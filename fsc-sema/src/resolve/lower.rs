use std::collections::{hash_map::Entry, HashMap};

use super::declarations::Declarations;
use super::error::ResolveError;
use crate::hir;
use crate::types::Ty;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics, Span};
use fsc_parse::ast;

// TODO: I don't like the split between Lowerer and FunctionLowerer check out if there is a
// cleaner and better understandable way

pub(super) fn lower(
    script: &ast::Script,
    declarations: &Declarations,
    diagnostics: &mut Diagnostics,
) -> hir::Script {
    Lowerer::new(declarations, diagnostics).lower_script(script)
}

struct Lowerer<'declarations, 'diagnostics> {
    declarations: &'declarations Declarations,
    diagnostics: &'diagnostics mut Diagnostics,
    expression_ids: ExprIdAllocator,
}

impl<'declarations, 'diagnostics> Lowerer<'declarations, 'diagnostics> {
    fn new(
        declarations: &'declarations Declarations,
        diagnostics: &'diagnostics mut Diagnostics,
    ) -> Self {
        Self {
            declarations,
            diagnostics,
            expression_ids: ExprIdAllocator::default(),
        }
    }

    fn lower_script(&mut self, script: &ast::Script) -> hir::Script {
        let mut items = PrimaryMap::new();
        let source_items = self.declarations.source_items().collect::<Vec<_>>();
        // debug only
        for (id, source_index) in source_items {
            let lowered_id = items.push(self.lower_item(&script.items[source_index]));
            debug_assert_eq!(lowered_id, id);
        }
        hir::Script { items }
    }

    fn lower_item(&mut self, item: &ast::Item) -> hir::Item {
        match item {
            ast::Item::FuncDef(function) => hir::Item {
                name: function.header.name.clone(),
                name_span: function.header.name_span,
                kind: hir::ItemKind::Function(hir::Function {
                    signature: self.lower_signature(&function.header),
                    kind: hir::FunctionKind::Defined {
                        exported: function.exported,
                        body: FunctionLowerer::new(
                            self.declarations,
                            self.diagnostics,
                            &mut self.expression_ids,
                        )
                        .lower(&function.header.params, &function.body),
                    },
                }),
            },
            ast::Item::ExternFuncDecl(extern_func) => hir::Item {
                name: extern_func.header.name.clone(),
                name_span: extern_func.header.name_span,
                kind: hir::ItemKind::Function(hir::Function {
                    signature: self.lower_signature(&extern_func.header),
                    kind: hir::FunctionKind::External,
                }),
            },
            ast::Item::ConstDecl(constant) => hir::Item {
                name: constant.name.clone(),
                name_span: constant.name_span,
                kind: hir::ItemKind::Constant(hir::Constant {
                    ty: Ty::from(&constant.ty),
                    type_span: constant.ty_span,
                    initializer: lower_expression(
                        &constant.initializer,
                        NameResolver::global(self.declarations),
                        self.diagnostics,
                        &mut self.expression_ids,
                    ),
                }),
            },
            ast::Item::ConfigDecl(config) => hir::Item {
                name: config.name.clone(),
                name_span: config.name_span,
                kind: hir::ItemKind::Config(hir::Config {
                    ty: Ty::from(&config.ty),
                    type_span: config.ty_span,
                }),
            },
        }
    }
    fn lower_signature(&mut self, header: &ast::FunctionHeader) -> hir::FunctionSignature {
        let mut parameter_names = HashMap::new();
        let mut parameters = Vec::with_capacity(header.params.len());

        for parameter in &header.params {
            if let Some(&original_span) = parameter_names.get(parameter.name.as_str()) {
                self.diagnostics
                    .emit_error(Diagnostic::from(ResolveError::DuplicateDeclaration {
                        name: parameter.name.clone(),
                        duplicate_span: parameter.name_span,
                        original_span,
                    }));
            } else {
                parameter_names.insert(parameter.name.as_str(), parameter.name_span);
            }
            parameters.push(hir::Parameter {
                name: parameter.name.clone(),
                name_span: parameter.name_span,
                ty: Ty::from(&parameter.ty),
                type_span: parameter.ty_span,
            });
        }

        hir::FunctionSignature {
            parameters,
            return_type: Ty::from(&header.ret_ty),
            return_type_span: header.ret_ty_span,
        }
    }
}

#[derive(Debug, Default)]
struct ExprIdAllocator {
    allocated: PrimaryMap<hir::ExprId, ()>,
}

impl ExprIdAllocator {
    fn allocate(&mut self) -> hir::ExprId {
        self.allocated.push(())
    }
}

struct FunctionLowerer<'declarations, 'diagnostics> {
    declarations: &'declarations Declarations,
    diagnostics: &'diagnostics mut Diagnostics,
    expression_ids: &'diagnostics mut ExprIdAllocator,
    locals: PrimaryMap<hir::LocalId, hir::Local>,
    scopes: LocalScopeStack,
}

impl<'declarations, 'diagnostics> FunctionLowerer<'declarations, 'diagnostics> {
    fn new(
        declarations: &'declarations Declarations,
        diagnostics: &'diagnostics mut Diagnostics,
        expression_ids: &'diagnostics mut ExprIdAllocator,
    ) -> Self {
        Self {
            declarations,
            diagnostics,
            expression_ids,
            locals: PrimaryMap::new(),
            scopes: LocalScopeStack::default(),
        }
    }

    fn lower(mut self, parameters: &[ast::Param], statements: &[ast::Stmt]) -> hir::Body {
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
            ast::StmtKind::ExprStmt(expression) => {
                hir::StmtKind::Expression(self.lower_expression(expression))
            }
            ast::StmtKind::Pause(expression) => {
                hir::StmtKind::Pause(self.lower_expression(expression))
            }
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
        lower_expression(
            expression,
            NameResolver::function(self.declarations, &self.scopes),
            self.diagnostics,
            self.expression_ids,
        )
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

#[derive(Debug, Clone, Copy)]
struct LocalBinding {
    id: hir::LocalId,
    declaration_span: Span,
}

#[derive(Debug, Default)]
struct LocalScopeStack {
    function: HashMap<String, LocalBinding>,
    blocks: Vec<HashMap<String, LocalBinding>>,
}

impl LocalScopeStack {
    fn enter_block(&mut self) {
        self.blocks.push(HashMap::new());
    }

    fn exit_block(&mut self) {
        self.blocks.pop();
    }

    fn try_declare(
        &mut self,
        name: &str,
        id: hir::LocalId,
        span: Span,
    ) -> Result<(), LocalBinding> {
        let scope = match self.blocks.last_mut() {
            Some(block) => block,
            None => &mut self.function,
        };
        let binding = LocalBinding {
            id,
            declaration_span: span,
        };
        match scope.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(binding);
                Ok(())
            }
            Entry::Occupied(entry) => Err(*entry.get()),
        }
    }

    fn resolve(&self, name: &str) -> Option<hir::LocalId> {
        self.blocks
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|binding| binding.id))
            .or_else(|| self.function.get(name).map(|binding| binding.id))
    }
}

#[derive(Clone, Copy)]
enum NameResolver<'a> {
    Global(&'a Declarations),
    Function {
        declarations: &'a Declarations,
        locals: &'a LocalScopeStack,
    },
}

impl<'a> NameResolver<'a> {
    const fn global(declarations: &'a Declarations) -> Self {
        Self::Global(declarations)
    }

    const fn function(declarations: &'a Declarations, locals: &'a LocalScopeStack) -> Self {
        Self::Function {
            declarations,
            locals,
        }
    }

    fn resolve(self, name: &str) -> Option<hir::ExprKind> {
        match self {
            Self::Global(declarations) => declarations.resolve(name).map(hir::ExprKind::ItemRef),
            Self::Function {
                declarations,
                locals,
            } => {
                if let Some(local) = locals.resolve(name) {
                    return Some(hir::ExprKind::Local(local));
                }
                declarations.resolve(name).map(hir::ExprKind::ItemRef)
            }
        }
    }
}

fn lower_expression(
    source: &ast::Expr,
    names: NameResolver<'_>,
    diagnostics: &mut Diagnostics,
    expression_ids: &mut ExprIdAllocator,
) -> hir::Expr {
    let kind = match &source.kind {
        ast::ExprKind::IntLit(value) => hir::ExprKind::Int(*value),
        ast::ExprKind::FloatLit(value) => hir::ExprKind::Float(*value),
        ast::ExprKind::BoolLit(value) => hir::ExprKind::Bool(*value),
        ast::ExprKind::StringLit(value) => hir::ExprKind::String(value.clone()),
        ast::ExprKind::Var(name) => names.resolve(name).unwrap_or_else(|| {
            hir::ExprKind::Error(diagnostics.emit_error(Diagnostic::from(
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
            base: Box::new(lower_expression(base, names, diagnostics, expression_ids)),
            member: member.clone(),
            member_span: *member_span,
        },
        ast::ExprKind::BinOp { op, lhs, rhs } => hir::ExprKind::Binary {
            op: lower_binary_operator(op),
            lhs: Box::new(lower_expression(lhs, names, diagnostics, expression_ids)),
            rhs: Box::new(lower_expression(rhs, names, diagnostics, expression_ids)),
        },
        ast::ExprKind::Unary { op, expr } => hir::ExprKind::Unary {
            op: lower_unary_operator(op),
            expr: Box::new(lower_expression(expr, names, diagnostics, expression_ids)),
        },
        ast::ExprKind::Call {
            callee,
            callee_span,
            args,
        } => hir::ExprKind::Call {
            callee: Box::new(hir::Expr {
                id: expression_ids.allocate(),
                kind: names.resolve(callee).unwrap_or_else(|| {
                    hir::ExprKind::Error(diagnostics.emit_error(Diagnostic::from(
                        ResolveError::UndeclaredName {
                            name: callee.clone(),
                            reference_span: *callee_span,
                        },
                    )))
                }),
                span: *callee_span,
            }),
            arguments: args
                .iter()
                .map(|argument| lower_expression(argument, names, diagnostics, expression_ids))
                .collect(),
        },
        ast::ExprKind::SysCall { args } => hir::ExprKind::RawSyscall {
            arguments: args
                .iter()
                .map(|argument| lower_expression(argument, names, diagnostics, expression_ids))
                .collect(),
        },
    };

    hir::Expr {
        id: expression_ids.allocate(),
        kind,
        span: source.span,
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
    }
}
