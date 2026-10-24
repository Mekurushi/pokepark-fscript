use super::super::builtin::{Builtin, BuiltinContext};
use super::super::error::ResolveError;
use super::names::NameResolver;
use crate::hir;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics};
use fsc_parse::ast;

#[derive(Debug, Default)]
pub(super) struct ExprIdAllocator {
    allocated: PrimaryMap<hir::ExprId, ()>,
}

impl ExprIdAllocator {
    pub(super) fn allocate(&mut self) -> hir::ExprId {
        self.allocated.push(())
    }
}

#[derive(Debug, Default)]
pub(super) struct ExprFieldIdAllocator {
    allocated: PrimaryMap<hir::ExprFieldId, ()>,
}

impl ExprFieldIdAllocator {
    fn allocate(&mut self) -> hir::ExprFieldId {
        self.allocated.push(())
    }
}

pub(super) struct ExpressionLowerer<'a> {
    names: NameResolver<'a>,
    diagnostics: &'a mut Diagnostics,
    expression_ids: &'a mut ExprIdAllocator,
    expr_field_ids: &'a mut ExprFieldIdAllocator,
}

impl<'a> ExpressionLowerer<'a> {
    pub(super) const fn new(
        names: NameResolver<'a>,
        diagnostics: &'a mut Diagnostics,
        expression_ids: &'a mut ExprIdAllocator,
        expr_field_ids: &'a mut ExprFieldIdAllocator,
    ) -> Self {
        Self {
            names,
            diagnostics,
            expression_ids,
            expr_field_ids,
        }
    }

    pub(super) fn lower(&mut self, source: &ast::Expr) -> hir::Expr {
        let kind = match &source.kind {
            ast::ExprKind::IntLit(value) => hir::ExprKind::Int(*value),
            ast::ExprKind::FloatLit(value) => hir::ExprKind::Float(*value),
            ast::ExprKind::BoolLit(value) => hir::ExprKind::Bool(*value),
            ast::ExprKind::StringLit(value) => hir::ExprKind::String(value.clone()),
            ast::ExprKind::Array { elements } => hir::ExprKind::Array {
                elements: elements.iter().map(|element| self.lower(element)).collect(),
            },
            ast::ExprKind::Aggregate { ty, fields } => hir::ExprKind::Aggregate {
                ty: crate::types::Ty::from(ty),
                fields: fields
                    .iter()
                    .map(|field| hir::ExprField {
                        id: self.expr_field_ids.allocate(),
                        name: field.name.clone(),
                        name_span: field.name_span,
                        value: self.lower(&field.value),
                    })
                    .collect(),
            },
            ast::ExprKind::Index { base, index } => hir::ExprKind::Index {
                base: Box::new(self.lower(base)),
                index: Box::new(self.lower(index)),
            },
            ast::ExprKind::Var(name) => self.names.resolve(name).unwrap_or_else(|| {
                hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(
                    ResolveError::UndeclaredName {
                        name: name.clone(),
                        reference_span: source.span,
                    },
                )))
            }),
            ast::ExprKind::Member {
                base,
                member,
                member_span,
            } => hir::ExprKind::Member {
                base: Box::new(self.lower(base)),
                member: member.clone(),
                member_span: *member_span,
            },
            ast::ExprKind::BinOp { op, lhs, rhs } => hir::ExprKind::Binary {
                op: lower_binary_operator(op),
                lhs: Box::new(self.lower(lhs)),
                rhs: Box::new(self.lower(rhs)),
            },
            ast::ExprKind::Unary { op, expr } => hir::ExprKind::Unary {
                op: lower_unary_operator(op),
                expr: Box::new(self.lower(expr)),
            },
            ast::ExprKind::Call {
                callee,
                callee_span,
                args,
            } => hir::ExprKind::Call {
                callee: Box::new(hir::Expr {
                    id: self.expression_ids.allocate(),
                    kind: self.names.resolve(callee).unwrap_or_else(|| {
                        hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(
                            ResolveError::UndeclaredName {
                                name: callee.clone(),
                                reference_span: *callee_span,
                            },
                        )))
                    }),
                    span: *callee_span,
                }),
                arguments: args.iter().map(|argument| self.lower(argument)).collect(),
            },
            ast::ExprKind::BuiltinCall {
                name,
                name_span,
                args,
            } => self.lower_builtin(name, *name_span, args, source.span),
        };

        hir::Expr {
            id: self.expression_ids.allocate(),
            kind,
            span: source.span,
        }
    }

    fn lower_builtin(
        &mut self,
        name: &str,
        name_span: fsc_diagnostics::Span,
        arguments: &[ast::Expr],
        invocation_span: fsc_diagnostics::Span,
    ) -> hir::ExprKind {
        let lowered_arguments = arguments
            .iter()
            .map(|argument| self.lower(argument))
            .collect::<Vec<_>>();

        let Some(builtin) = Builtin::resolve(name) else {
            return hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(
                ResolveError::UnknownBuiltin {
                    name: name.to_owned(),
                    name_span,
                },
            )));
        };

        if let Err(error) = builtin.validate_argument_count(arguments.len(), invocation_span) {
            return hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(error)));
        }

        if !builtin.supports_context(BuiltinContext::Value) {
            return hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(
                ResolveError::InvalidBuiltinContext {
                    name: builtin.name(),
                    context: BuiltinContext::Value,
                    invocation_span,
                },
            )));
        }

        match builtin {
            Builtin::Syscall => hir::ExprKind::RawSyscall {
                arguments: lowered_arguments,
            },
            Builtin::Pause => unreachable!("statement builtin reached value lowering"),
        }
    }
}

const fn lower_binary_operator(operator: &ast::BinOp) -> hir::BinaryOp {
    match operator {
        ast::BinOp::Add => hir::BinaryOp::Add,
        ast::BinOp::Sub => hir::BinaryOp::Sub,
        ast::BinOp::Mul => hir::BinaryOp::Mul,
        ast::BinOp::Div => hir::BinaryOp::Div,
        ast::BinOp::Eq => hir::BinaryOp::Eq,
        ast::BinOp::Neq => hir::BinaryOp::NotEq,
        ast::BinOp::Lt => hir::BinaryOp::Less,
        ast::BinOp::Gt => hir::BinaryOp::Greater,
        ast::BinOp::Le => hir::BinaryOp::LessEq,
        ast::BinOp::Ge => hir::BinaryOp::GreaterEq,
        ast::BinOp::And => hir::BinaryOp::And,
        ast::BinOp::Or => hir::BinaryOp::Or,
    }
}

const fn lower_unary_operator(operator: &ast::UnaryOp) -> hir::UnaryOp {
    match operator {
        ast::UnaryOp::Not => hir::UnaryOp::Not,
        ast::UnaryOp::Neg => hir::UnaryOp::Neg,
        ast::UnaryOp::AddressOf => hir::UnaryOp::AddressOf,
    }
}
