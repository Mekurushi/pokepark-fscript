use super::super::builtin::{Builtin, BuiltinContext};
use super::super::{declarations::Declarations, error::ResolveError};
use super::expression::{ExprFieldIdAllocator, ExprIdAllocator, ExpressionLowerer};
use super::names::{LocalScopeStack, NameResolver};
use crate::{hir, types::Ty};
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics, Span};
use fsc_parse::ast;

pub(super) struct BodyLowerer<'declarations, 'diagnostics> {
    declarations: &'declarations Declarations,
    diagnostics: &'diagnostics mut Diagnostics,
    expression_ids: &'diagnostics mut ExprIdAllocator,
    expr_field_ids: &'diagnostics mut ExprFieldIdAllocator,
    locals: PrimaryMap<hir::LocalId, hir::Local>,
    scopes: LocalScopeStack,
}

impl<'declarations, 'diagnostics> BodyLowerer<'declarations, 'diagnostics> {
    pub(super) fn new(
        declarations: &'declarations Declarations,
        diagnostics: &'diagnostics mut Diagnostics,
        expression_ids: &'diagnostics mut ExprIdAllocator,
        expr_field_ids: &'diagnostics mut ExprFieldIdAllocator,
    ) -> Self {
        Self {
            declarations,
            diagnostics,
            expression_ids,
            expr_field_ids,
            locals: PrimaryMap::new(),
            scopes: LocalScopeStack::default(),
        }
    }

    pub(super) fn lower(
        mut self,
        parameters: &[ast::Param],
        statements: &[ast::Stmt],
    ) -> hir::Body {
        let mut parameter_locals = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let local = self.declare_parameter(
                &parameter.name,
                parameter.name_span,
                &parameter.ty,
                parameter.ty_span,
                index,
            );
            parameter_locals.push(local);
        }

        let statements = self.lower_statements(statements);
        hir::Body {
            parameter_locals,
            locals: self.locals,
            block: hir::Block { statements },
        }
    }

    fn lower_statements(&mut self, statements: &[ast::Stmt]) -> Vec<hir::Stmt> {
        statements
            .iter()
            .map(|statement| self.lower_statement(statement))
            .collect()
    }

    fn lower_statement(&mut self, source: &ast::Stmt) -> hir::Stmt {
        let kind = match &source.kind {
            ast::StmtKind::Return(value) => hir::StmtKind::Return(
                value
                    .as_ref()
                    .map(|expression| self.lower_expression(expression)),
            ),
            ast::StmtKind::Break => hir::StmtKind::Break,
            ast::StmtKind::VarDecl {
                name,
                name_span,
                ty,
                ty_span,
                init,
            } => {
                let initializer = init
                    .as_ref()
                    .map(|expression| self.lower_expression(expression));
                let local =
                    self.declare_local(name, *name_span, ty, *ty_span, hir::LocalKind::Variable);
                hir::StmtKind::VariableDeclaration { local, initializer }
            }
            ast::StmtKind::ConstDecl(constant) => {
                let initializer = self.lower_expression(&constant.initializer);
                let local = self.declare_local(
                    &constant.name,
                    constant.name_span,
                    &constant.ty,
                    constant.ty_span,
                    hir::LocalKind::Constant,
                );
                hir::StmtKind::ConstantDeclaration { local, initializer }
            }
            ast::StmtKind::Assign { target, expr } => hir::StmtKind::Assign {
                target: self.lower_expression(target),
                value: self.lower_expression(expr),
            },
            ast::StmtKind::ExprStmt(expression) => self.lower_expression_statement(expression),
            ast::StmtKind::If {
                cond,
                then_body,
                else_body,
            } => hir::StmtKind::If {
                condition: self.lower_expression(cond),
                then_block: self.lower_block(then_body),
                else_block: else_body
                    .as_ref()
                    .map(|statements| self.lower_block(statements)),
            },
            ast::StmtKind::While { cond, body } => hir::StmtKind::While {
                condition: self.lower_expression(cond),
                body: self.lower_block(body),
            },
        };

        hir::Stmt {
            kind,
            span: source.span,
        }
    }

    fn lower_block(&mut self, statements: &[ast::Stmt]) -> hir::Block {
        self.scopes.enter_block();
        let statements = self.lower_statements(statements);
        self.scopes.exit_block();
        hir::Block { statements }
    }

    fn lower_expression(&mut self, expression: &ast::Expr) -> hir::Expr {
        ExpressionLowerer::new(
            NameResolver::function(self.declarations, &self.scopes),
            self.diagnostics,
            self.expression_ids,
            self.expr_field_ids,
        )
        .lower(expression)
    }

    fn lower_expression_statement(&mut self, expression: &ast::Expr) -> hir::StmtKind {
        let ast::ExprKind::BuiltinCall { name, args, .. } = &expression.kind else {
            return hir::StmtKind::Expression(self.lower_expression(expression));
        };

        let Some(builtin) = Builtin::resolve(name) else {
            return hir::StmtKind::Expression(self.lower_expression(expression));
        };
        if !builtin.supports_context(BuiltinContext::Statement) {
            return hir::StmtKind::Expression(hir::Expr {
                id: self.expression_ids.allocate(),
                kind: hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(
                    ResolveError::InvalidBuiltinContext {
                        name: builtin.name(),
                        context: BuiltinContext::Statement,
                        invocation_span: expression.span,
                    },
                ))),
                span: expression.span,
            });
        }

        if let Err(error) = builtin.validate_argument_count(args.len(), expression.span) {
            return hir::StmtKind::Expression(hir::Expr {
                id: self.expression_ids.allocate(),
                kind: hir::ExprKind::Error(self.diagnostics.emit_error(Diagnostic::from(error))),
                span: expression.span,
            });
        }

        match builtin {
            Builtin::Pause => hir::StmtKind::Pause(self.lower_expression(&args[0])),
            Builtin::Syscall => hir::StmtKind::Expression(self.lower_expression(expression)),
        }
    }

    fn declare_local(
        &mut self,
        name: &str,
        name_span: Span,
        ty: &ast::Ty,
        type_span: Span,
        kind: hir::LocalKind,
    ) -> hir::LocalId {
        let id = self.create_local(name, name_span, ty, type_span, kind);
        if let Err(original) = self.scopes.try_declare(name, id, name_span) {
            self.diagnostics
                .emit_error(Diagnostic::from(ResolveError::DuplicateDeclaration {
                    name: name.to_owned(),
                    duplicate_span: name_span,
                    original_span: original.declaration_span,
                }));
        }
        id
    }

    fn declare_parameter(
        &mut self,
        name: &str,
        name_span: Span,
        ty: &ast::Ty,
        type_span: Span,
        index: usize,
    ) -> hir::LocalId {
        let id = self.create_local(
            name,
            name_span,
            ty,
            type_span,
            hir::LocalKind::Parameter { index },
        );
        let _ = self.scopes.try_declare(name, id, name_span);
        id
    }

    fn create_local(
        &mut self,
        name: &str,
        name_span: Span,
        ty: &ast::Ty,
        type_span: Span,
        kind: hir::LocalKind,
    ) -> hir::LocalId {
        self.locals.push(hir::Local {
            name: name.to_owned(),
            name_span,
            ty: Ty::from(ty),
            type_span,
            kind,
        })
    }
}
