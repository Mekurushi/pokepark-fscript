use crate::bind::BoundScript;
use crate::checked::{CheckedFunction, CheckedScript};
use crate::error::{SemaError, SemaResult};
use crate::hir::{self, LocalId, Place, ProjectionElem, Vec3Field};
use crate::resolve::Resolutions;
use crate::symbol::{ConstValue, SymbolId, SymbolKind, SymbolTable};
use crate::types::Ty;
use fsc_parse::ast::{self, Expr, Stmt};
use std::collections::HashMap;

struct LocalMap {
    declarations: Vec<hir::Local>,
    by_symbol: HashMap<SymbolId, LocalId>,
}

impl LocalMap {
    fn new(resolutions: &Resolutions, symbols: &SymbolTable) -> Self {
        let mut declarations = Vec::new();
        let mut by_symbol = HashMap::new();

        for &symbol_id in resolutions.frame_symbols() {
            let symbol = symbols.get(symbol_id);
            let kind = match symbol.kind {
                SymbolKind::Param { .. } => hir::LocalKind::Parameter,
                SymbolKind::Local => hir::LocalKind::Variable,
                SymbolKind::Const { .. } | SymbolKind::Config | SymbolKind::Function { .. } => {
                    continue;
                }
            };
            let local = LocalId::new(declarations.len() as u32);
            declarations.push(hir::Local {
                id: local,
                ty: symbol.ty.clone(),
                kind,
            });
            by_symbol.insert(symbol_id, local);
        }

        Self {
            declarations,
            by_symbol,
        }
    }

    fn local_for_symbol(
        &self,
        symbol: SymbolId,
        name: &str,
        reference_span: fsc_diagnostics::Span,
    ) -> SemaResult<LocalId> {
        self.by_symbol
            .get(&symbol)
            .copied()
            .ok_or_else(|| SemaError::MissingLocalIdentity {
                name: name.to_owned(),
                reference_span,
            })
    }

    fn into_declarations(self) -> Vec<hir::Local> {
        self.declarations
    }
}

struct LoweringContext<'a> {
    checked: &'a CheckedFunction,
    symbols: &'a SymbolTable,
    locals: LocalMap,
}

impl<'a> LoweringContext<'a> {
    fn new(checked: &'a CheckedFunction, symbols: &'a SymbolTable) -> Self {
        Self {
            checked,
            symbols,
            locals: LocalMap::new(&checked.resolutions, symbols),
        }
    }

    fn lower_fn(self) -> SemaResult<hir::FuncDef> {
        let function = self.symbols.get(self.checked.symbol);
        let SymbolKind::Function { ret_ty, .. } = &function.kind else {
            return Err(SemaError::MissingFunctionSignature {
                name: function.name.clone(),
                declaration_span: function.name_span,
            });
        };
        let ret_ty = ret_ty.clone();
        let mut body = self.lower_stmts(&self.checked.function.body)?;

        if ret_ty == Ty::Void && stmts_can_fall_through(&body) {
            body.push(hir::Stmt::ReturnVoid);
        }

        Ok(hir::FuncDef {
            name: self.checked.function.header.name.clone(),
            exported: self.checked.function.exported,
            ret_ty,
            locals: self.locals.into_declarations(),
            body,
        })
    }

    fn lower_stmts(&self, stmts: &[Stmt]) -> SemaResult<Vec<hir::Stmt>> {
        stmts.iter().map(|stmt| self.lower_stmt(stmt)).collect()
    }

    fn lower_stmt(&self, stmt: &Stmt) -> SemaResult<hir::Stmt> {
        match &stmt.kind {
            ast::StmtKind::Return(expr) => match expr {
                Some(expr) => Ok(hir::Stmt::Return(self.lower_expr(expr)?)),
                None => Ok(hir::Stmt::ReturnVoid),
            },
            ast::StmtKind::Break => Ok(hir::Stmt::Break),
            ast::StmtKind::VarDecl { init, .. } => {
                let hir_init = init
                    .as_ref()
                    .map(|expr| self.lower_expr(expr))
                    .transpose()?;
                let symbol_id = self.checked.resolutions.symbol(stmt.id);
                let symbol = self.symbols.get(symbol_id);
                let local = self
                    .locals
                    .local_for_symbol(symbol_id, &symbol.name, stmt.span)?;

                Ok(hir::Stmt::VarDecl {
                    local,
                    init: hir_init,
                })
            }
            ast::StmtKind::Assign { target, expr } => Ok(hir::Stmt::Assign {
                target: self.lower_place(target)?,
                value: self.lower_expr(expr)?,
            }),
            ast::StmtKind::If {
                cond,
                then_body,
                else_body,
            } => Ok(hir::Stmt::If {
                cond: self.lower_expr(cond)?,
                then_body: self.lower_stmts(then_body)?,
                else_body: else_body
                    .as_ref()
                    .map(|body| self.lower_stmts(body))
                    .transpose()?,
            }),
            ast::StmtKind::While { cond, body } => Ok(hir::Stmt::While {
                cond: self.lower_expr(cond)?,
                body: self.lower_stmts(body)?,
            }),
            ast::StmtKind::ExprStmt(expr) => Ok(hir::Stmt::ExprStmt(self.lower_expr(expr)?)),
            ast::StmtKind::Pause(expr) => Ok(hir::Stmt::Pause(self.lower_expr(expr)?)),
        }
    }

    fn lower_expr(&self, expr: &Expr) -> SemaResult<hir::Expr> {
        match &expr.kind {
            ast::ExprKind::IntLit(v) => Ok(hir::Expr::IntLit {
                value: *v,
                ty: Ty::Int,
            }),
            ast::ExprKind::FloatLit(v) => Ok(hir::Expr::FloatLit {
                value: *v,
                ty: Ty::Float,
            }),
            ast::ExprKind::StringLit(v) => Ok(hir::Expr::StrLit {
                value: v.clone(),
                ty: Ty::Int,
            }),

            ast::ExprKind::BoolLit(v) => Ok(hir::Expr::BoolLit {
                value: *v,
                ty: Ty::Bool,
            }),

            ast::ExprKind::Var(_name) => {
                let symbol_id = self.checked.resolutions.symbol(expr.id);
                let symbol = self.symbols.get(symbol_id);
                match &symbol.kind {
                    SymbolKind::Const { value, .. } => Ok(lower_const_value(value)),
                    SymbolKind::Config => Err(SemaError::MissingConfigValue {
                        name: symbol.name.clone(),
                        declaration_span: symbol.name_span,
                    }),
                    SymbolKind::Param { .. } | SymbolKind::Local => {
                        let local =
                            self.locals
                                .local_for_symbol(symbol_id, &symbol.name, expr.span)?;
                        Ok(hir::Expr::Load {
                            place: Place::new(local),
                            ty: symbol.ty.clone(),
                        })
                    }
                    SymbolKind::Function { .. } => Err(SemaError::NotAValue {
                        name: symbol.name.clone(),
                        reference_span: expr.span,
                        declaration_span: symbol.name_span,
                    }),
                }
            }

            ast::ExprKind::Member { .. } => {
                let ty = self
                    .checked
                    .expression_types
                    .get(expr.id)
                    .cloned()
                    .ok_or(SemaError::MissingExpressionType { span: expr.span })?;
                Ok(hir::Expr::Load {
                    place: self.lower_place(expr)?,
                    ty,
                })
            }

            ast::ExprKind::BinOp { op, lhs, rhs } => {
                let ty = self
                    .checked
                    .expression_types
                    .get(expr.id)
                    .cloned()
                    .ok_or(SemaError::MissingExpressionType { span: expr.span })?;
                Ok(hir::Expr::BinOp {
                    op: lower_binop(op),
                    lhs: Box::new(self.lower_expr(lhs)?),
                    rhs: Box::new(self.lower_expr(rhs)?),
                    ty,
                })
            }

            ast::ExprKind::Unary { op, expr: inner } => {
                let ty = self
                    .checked
                    .expression_types
                    .get(expr.id)
                    .cloned()
                    .ok_or(SemaError::MissingExpressionType { span: expr.span })?;
                Ok(hir::Expr::Unary {
                    op: lower_unaryop(op),
                    expr: Box::new(self.lower_expr(inner)?),
                    ty,
                })
            }

            ast::ExprKind::Call { callee, args, .. } => {
                let ty = self
                    .checked
                    .expression_types
                    .get(expr.id)
                    .cloned()
                    .ok_or(SemaError::MissingExpressionType { span: expr.span })?;

                Ok(hir::Expr::Call {
                    callee: callee.clone(),
                    args: args
                        .iter()
                        .map(|argument| self.lower_expr(argument))
                        .collect::<Result<_, _>>()?,
                    ty,
                })
            }
            ast::ExprKind::SysCall { args } => {
                let (page, func) = extract_syscall(args)?;
                let lowered_args: Vec<_> = args[2..]
                    .iter()
                    .map(
                        |argument| match self.checked.expression_types.get(argument.id) {
                            Some(Ty::Vec3) => {
                                self.lower_place(argument).map(hir::SysCallArg::StackRef)
                            }
                            _ => self.lower_expr(argument).map(hir::SysCallArg::Value),
                        },
                    )
                    .collect::<Result<_, _>>()?;

                let subtype = lowered_args.len() as u8;

                Ok(hir::Expr::SysCall {
                    page,
                    func,
                    subtype,
                    args: lowered_args,
                    ty: self
                        .checked
                        .expression_types
                        .get(expr.id)
                        .cloned()
                        .ok_or(SemaError::MissingExpressionType { span: expr.span })?,
                })
            }
        }
    }

    fn lower_place(&self, expr: &Expr) -> SemaResult<Place> {
        match &expr.kind {
            ast::ExprKind::Var(_) => {
                let symbol_id = self.checked.resolutions.symbol(expr.id);
                let symbol = self.symbols.get(symbol_id);
                let local = self
                    .locals
                    .local_for_symbol(symbol_id, &symbol.name, expr.span)?;
                Ok(Place::new(local))
            }
            ast::ExprKind::Member {
                base,
                member,
                member_span,
            } => {
                let field =
                    Vec3Field::from_name(member).ok_or_else(|| SemaError::InvalidMember {
                        ty: Ty::Vec3,
                        member: member.clone(),
                        member_span: *member_span,
                    })?;
                Ok(self
                    .lower_place(base)?
                    .project(ProjectionElem::Vec3Field(field)))
            }
            _ => Err(SemaError::InvalidAssignmentTarget {
                target_span: expr.span,
            }),
        }
    }
}

pub(crate) fn lower_script(bound: BoundScript) -> SemaResult<hir::Script> {
    let BoundScript(checked) = bound;
    let CheckedScript {
        functions, symbols, ..
    } = checked;
    let items = functions
        .iter()
        .map(|checked| {
            LoweringContext::new(checked, &symbols)
                .lower_fn()
                .map(hir::Item::FuncDef)
        })
        .collect::<SemaResult<_>>()?;

    Ok(hir::Script { items })
}

fn stmts_can_fall_through(stmts: &[hir::Stmt]) -> bool {
    stmts.iter().all(stmt_can_fall_through)
}

fn stmt_can_fall_through(stmt: &hir::Stmt) -> bool {
    match stmt {
        hir::Stmt::Return(_) | hir::Stmt::ReturnVoid | hir::Stmt::Break => false,
        hir::Stmt::If {
            then_body,
            else_body: Some(else_body),
            ..
        } => stmts_can_fall_through(then_body) || stmts_can_fall_through(else_body),
        hir::Stmt::If {
            else_body: None, ..
        }
        | hir::Stmt::While { .. }
        | hir::Stmt::VarDecl { .. }
        | hir::Stmt::Assign { .. }
        | hir::Stmt::ExprStmt(_)
        | hir::Stmt::Pause(_) => true,
    }
}

fn lower_const_value(value: &ConstValue) -> hir::Expr {
    match value {
        ConstValue::Int(value) => hir::Expr::IntLit {
            value: *value,
            ty: Ty::Int,
        },
        ConstValue::Float(value) => hir::Expr::FloatLit {
            value: *value,
            ty: Ty::Float,
        },
        ConstValue::Bool(value) => hir::Expr::BoolLit {
            value: *value,
            ty: Ty::Bool,
        },
        ConstValue::Str(value) => hir::Expr::StrLit {
            value: value.clone(),
            ty: Ty::Int,
        },
    }
}

pub fn extract_syscall(args: &[Expr]) -> SemaResult<(u8, u16)> {
    if args.len() < 3 {
        todo!("Syscall called with too few arguments");
    }

    let page = extract_syscall_u8(&args[0])?;
    let func = extract_syscall_u16(&args[1])?;

    Ok((page, func))
}
fn extract_syscall_u8(expr: &Expr) -> SemaResult<u8> {
    match &expr.kind {
        ast::ExprKind::IntLit(n) => u8::try_from(*n).map_err(|_| todo!("out of range")),
        _ => todo!("unsupported expr kind"),
    }
}

fn extract_syscall_u16(expr: &Expr) -> SemaResult<u16> {
    match &expr.kind {
        ast::ExprKind::IntLit(n) => u16::try_from(*n).map_err(|_| todo!("out of range")),
        _ => todo!("unsupported expr kind"),
    }
}

fn lower_unaryop(op: &ast::UnaryOp) -> hir::UnaryOp {
    match op {
        ast::UnaryOp::Not => hir::UnaryOp::Not,
        ast::UnaryOp::Neg => hir::UnaryOp::Neg,
    }
}

fn lower_binop(op: &ast::BinOp) -> hir::BinOp {
    match op {
        ast::BinOp::Add => hir::BinOp::Add,
        ast::BinOp::Sub => hir::BinOp::Sub,
        ast::BinOp::Mul => hir::BinOp::Mul,
        ast::BinOp::Div => hir::BinOp::Div,

        // comparison
        ast::BinOp::Lt => hir::BinOp::Lt,
        ast::BinOp::Gt => hir::BinOp::Gt,
        ast::BinOp::Le => hir::BinOp::Le,
        ast::BinOp::Ge => hir::BinOp::Ge,
        ast::BinOp::Eq => hir::BinOp::Eq,
        ast::BinOp::Neq => hir::BinOp::Neq,
        // logical
        ast::BinOp::And => hir::BinOp::And,
        ast::BinOp::Or => hir::BinOp::Or,
    }
}
