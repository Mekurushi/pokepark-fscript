use crate::checked::CheckedFunction;
use crate::error::{SemaError, SemaResult};
use crate::resolve::{Resolutions, ResolvedFunction};
use crate::symbol::{SymbolId, SymbolKind, SymbolTable};
use crate::types::Ty;
use fsc_diagnostics::Span;
use fsc_parse::ast::{self, BinOp, Expr, ExprKind, NodeId, UnaryOp};
use std::collections::HashMap;

pub(crate) struct ExpressionTypes {
    by_node: HashMap<NodeId, Ty>,
}

impl ExpressionTypes {
    fn new() -> Self {
        Self {
            by_node: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, node: NodeId) -> Option<&Ty> {
        self.by_node.get(&node)
    }

    fn insert(&mut self, node: NodeId, ty: Ty) {
        self.by_node.insert(node, ty);
    }
}

struct TypeChecker<'a> {
    resolutions: &'a Resolutions,
    symbols: &'a SymbolTable,
    expression_types: ExpressionTypes,
}

impl<'a> TypeChecker<'a> {
    fn new(resolutions: &'a Resolutions, symbols: &'a SymbolTable) -> Self {
        Self {
            resolutions,
            symbols,
            expression_types: ExpressionTypes::new(),
        }
    }

    fn check_stmts(
        &mut self,
        stmts: &[ast::Stmt],
        ret_ty: &Ty,
        ret_ty_span: Span,
    ) -> SemaResult<()> {
        for stmt in stmts {
            self.check_stmt(stmt, ret_ty, ret_ty_span)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt, ret_ty: &Ty, ret_ty_span: Span) -> SemaResult<()> {
        match &stmt.kind {
            ast::StmtKind::Return(expr) => match expr {
                Some(expr) => {
                    let found = self.check_expr(expr)?;
                    check_return(ret_ty, &found, expr.span, ret_ty_span)
                }
                None => check_void_return(ret_ty, stmt.span, ret_ty_span),
            },
            ast::StmtKind::Break => Ok(()),
            ast::StmtKind::VarDecl {
                ty, ty_span, init, ..
            } => {
                if let Some(expr) = init {
                    let found = self.check_expr(expr)?;
                    check_assignable(&Ty::from(ty), &found, expr.span, Some(*ty_span))?;
                }
                Ok(())
            }
            ast::StmtKind::Assign { target, expr } => {
                let symbol = self.symbols.get(self.place_symbol(target)?);
                match symbol.kind {
                    SymbolKind::Const { .. } => {
                        return Err(SemaError::AssignmentToConstant {
                            name: symbol.name.clone(),
                            assignment_span: target.span,
                            declaration_span: symbol.name_span,
                        });
                    }
                    SymbolKind::Config => {
                        return Err(SemaError::AssignmentToConfig {
                            name: symbol.name.clone(),
                            assignment_span: target.span,
                            declaration_span: symbol.name_span,
                        });
                    }
                    _ => {}
                }
                let expected_span = symbol.type_span;
                let declared = self.check_place(target)?;
                let found = self.check_expr(expr)?;
                check_assignable(&declared, &found, expr.span, Some(expected_span))
            }
            ast::StmtKind::If {
                cond,
                then_body,
                else_body,
            } => {
                let cond_ty = self.check_expr(cond)?;
                check_condition(&cond_ty, cond.span)?;
                self.check_stmts(then_body, ret_ty, ret_ty_span)?;
                if let Some(else_stmts) = else_body {
                    self.check_stmts(else_stmts, ret_ty, ret_ty_span)?;
                }
                Ok(())
            }
            ast::StmtKind::While { cond, body } => {
                let cond_ty = self.check_expr(cond)?;
                check_condition(&cond_ty, cond.span)?;
                self.check_stmts(body, ret_ty, ret_ty_span)
            }
            ast::StmtKind::ExprStmt(expr) => {
                let ty = self.check_expr(expr)?;
                if ty != Ty::Void && !matches!(expr.kind, ast::ExprKind::Call { .. }) {
                    // TODO: emit warning: "expression result unused"
                }
                Ok(())
            }
            ast::StmtKind::Pause(expr) => {
                let found = self.check_expr(expr)?;
                check_assignable(&Ty::Int, &found, expr.span, None)
            }
        }
    }

    fn place_symbol(&self, expr: &Expr) -> SemaResult<SymbolId> {
        match &expr.kind {
            ExprKind::Var(_) => Ok(self.resolutions.symbol(expr.id)),
            ExprKind::Member { base, .. } => self.place_symbol(base),
            _ => Err(SemaError::InvalidAssignmentTarget {
                target_span: expr.span,
            }),
        }
    }

    fn check_place(&mut self, expr: &Expr) -> SemaResult<Ty> {
        let ty = match &expr.kind {
            ExprKind::Var(_) => {
                let symbol = self.symbols.get(self.resolutions.symbol(expr.id));
                if matches!(symbol.kind, SymbolKind::Function { .. }) {
                    return Err(SemaError::NotAValue {
                        name: symbol.name.clone(),
                        reference_span: expr.span,
                        declaration_span: symbol.name_span,
                    });
                }
                symbol.ty.clone()
            }
            ExprKind::Member {
                base,
                member,
                member_span,
            } => {
                let base_ty = self.check_place(base)?;
                if base_ty != Ty::Vec3 || !matches!(member.as_str(), "x" | "y" | "z") {
                    return Err(SemaError::InvalidMember {
                        ty: base_ty,
                        member: member.clone(),
                        member_span: *member_span,
                    });
                }
                Ty::Float
            }
            _ => {
                return Err(SemaError::InvalidAssignmentTarget {
                    target_span: expr.span,
                });
            }
        };
        self.expression_types.insert(expr.id, ty.clone());
        Ok(ty)
    }

    fn check_expr(&mut self, expr: &Expr) -> SemaResult<Ty> {
        let ty = match &expr.kind {
            ExprKind::IntLit(_) => Ty::Int,
            ExprKind::FloatLit(_) => Ty::Float,
            ExprKind::BoolLit(_) => Ty::Bool,
            ExprKind::StringLit(_) => Ty::Str,
            ExprKind::Var(_) => {
                let symbol = self.symbols.get(self.resolutions.symbol(expr.id));
                if matches!(symbol.kind, SymbolKind::Function { .. }) {
                    return Err(SemaError::NotAValue {
                        name: symbol.name.clone(),
                        reference_span: expr.span,
                        declaration_span: symbol.name_span,
                    });
                }
                if symbol.ty == Ty::Vec3 {
                    return Err(SemaError::UnsupportedValueType {
                        ty: symbol.ty.clone(),
                        span: expr.span,
                    });
                }
                symbol.ty.clone()
            }
            ExprKind::Member { .. } => self.check_place(expr)?,
            ExprKind::BinOp { op, lhs, rhs } => self.check_binop(op, lhs, rhs)?,
            ExprKind::Unary { op, expr } => self.check_unary(op, expr)?,
            ExprKind::Call {
                args, callee_span, ..
            } => self.check_call(expr, args, *callee_span)?,
            ExprKind::SysCall { args } => {
                // TODO: figure out how to type out Syscalls
                for argument in &args[2..] {
                    if let ExprKind::Var(_) = argument.kind {
                        let symbol = self.symbols.get(self.resolutions.symbol(argument.id));
                        if symbol.ty == Ty::Vec3 {
                            self.expression_types.insert(argument.id, Ty::Vec3);
                            continue;
                        }
                    }
                    self.check_expr(argument)?;
                }
                Ty::Int
            }
        };

        self.expression_types.insert(expr.id, ty.clone());
        Ok(ty)
    }

    fn check_call(&mut self, expr: &Expr, args: &[Expr], callee_span: Span) -> SemaResult<Ty> {
        let symbol = self.symbols.get(self.resolutions.symbol(expr.id)).clone();
        let SymbolKind::Function { ret_ty, params } = symbol.kind else {
            return Err(SemaError::NotCallable {
                name: symbol.name,
                callee_span,
                declaration_span: symbol.name_span,
            });
        };

        if args.len() != params.len() {
            return Err(SemaError::ArgumentCountMismatch {
                name: symbol.name,
                expected: params.len(),
                found: args.len(),
                call_span: expr.span,
                declaration_span: symbol.name_span,
            });
        }

        for (argument, parameter) in args.iter().zip(params) {
            let found = self.check_expr(argument)?;
            check_assignable(
                &parameter.ty,
                &found,
                argument.span,
                Some(parameter.type_span),
            )?;
        }

        Ok(ret_ty)
    }

    fn check_binop(&mut self, op: &BinOp, lhs: &Expr, rhs: &Expr) -> SemaResult<Ty> {
        let lhs_ty = self.check_expr(lhs)?;
        let rhs_ty = self.check_expr(rhs)?;

        if lhs_ty == Ty::Void || rhs_ty == Ty::Void {
            return Err(SemaError::VoidInValuePosition {
                span: if lhs_ty == Ty::Void {
                    lhs.span
                } else {
                    rhs.span
                },
            });
        }
        check_assignable(&lhs_ty, &rhs_ty, rhs.span, Some(lhs.span))?;

        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => Ok(lhs_ty),
            BinOp::Eq
            | BinOp::Neq
            | BinOp::Lt
            | BinOp::Gt
            | BinOp::Le
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or => Ok(Ty::Bool),
        }
    }

    fn check_unary(&mut self, op: &UnaryOp, expr: &Expr) -> SemaResult<Ty> {
        let ty = self.check_expr(expr)?;
        match op {
            UnaryOp::Neg => {
                if !matches!(ty, Ty::Int | Ty::Float) {
                    return Err(SemaError::TypeMismatch {
                        expected: Ty::Int,
                        found: ty,
                        span: expr.span,
                        expected_span: None,
                    });
                }
                Ok(ty)
            }
            UnaryOp::Not => {
                if ty == Ty::Void {
                    return Err(SemaError::VoidInValuePosition { span: expr.span });
                }
                Ok(Ty::Bool)
            }
        }
    }
}

pub(crate) fn check(
    resolved_functions: Vec<(ast::FuncDef, ResolvedFunction)>,
    symbols: &SymbolTable,
) -> SemaResult<Vec<CheckedFunction>> {
    for (_, symbol) in symbols.iter() {
        match &symbol.kind {
            SymbolKind::Config if matches!(symbol.ty, Ty::Void | Ty::Vec3) => {
                return Err(SemaError::InvalidConfigType {
                    ty: symbol.ty.clone(),
                    type_span: symbol.type_span,
                });
            }
            SymbolKind::Const { value, value_span } => {
                if matches!(symbol.ty, Ty::Void | Ty::Vec3) {
                    return Err(SemaError::InvalidConstantType {
                        ty: symbol.ty.clone(),
                        type_span: symbol.type_span,
                    });
                }
                check_assignable(&symbol.ty, &value.ty(), *value_span, Some(symbol.type_span))?;
            }
            SymbolKind::Function { ret_ty, params } => {
                // TODO: check out if it's possible to return vec3 cleanly
                if *ret_ty == Ty::Vec3 {
                    return Err(SemaError::InvalidReturnType {
                        ty: ret_ty.clone(),
                        type_span: symbol.type_span,
                    });
                }
                for parameter in params {
                    if matches!(parameter.ty, Ty::Void | Ty::Vec3) {
                        return Err(SemaError::InvalidParameterType {
                            ty: parameter.ty.clone(),
                            type_span: parameter.type_span,
                        });
                    }
                }
            }
            SymbolKind::Local if symbol.ty == Ty::Void => {
                return Err(SemaError::InvalidLocalType {
                    ty: symbol.ty.clone(),
                    type_span: symbol.type_span,
                });
            }
            SymbolKind::Param { .. } | SymbolKind::Local | SymbolKind::Config => {}
        }
    }

    resolved_functions
        .into_iter()
        .map(|(function, resolved)| {
            let ResolvedFunction {
                symbol,
                resolutions,
            } = resolved;
            let expression_types = {
                let mut checker = TypeChecker::new(&resolutions, symbols);
                checker.check_stmts(
                    &function.body,
                    &Ty::from(&function.header.ret_ty),
                    function.header.ret_ty_span,
                )?;
                checker.expression_types
            };

            Ok(CheckedFunction {
                function,
                symbol,
                resolutions,
                expression_types,
            })
        })
        .collect()
}

fn check_assignable(
    declared: &Ty,
    found: &Ty,
    span: Span,
    expected_span: Option<Span>,
) -> SemaResult<()> {
    if declared != found {
        return Err(SemaError::TypeMismatch {
            expected: declared.clone(),
            found: found.clone(),
            span,
            expected_span,
        });
    }
    Ok(())
}

fn check_return(ret_ty: &Ty, found: &Ty, span: Span, return_type_span: Span) -> SemaResult<()> {
    if ret_ty != found {
        return Err(SemaError::ReturnTypeMismatch {
            expected: ret_ty.clone(),
            found: found.clone(),
            span,
            return_type_span,
        });
    }
    Ok(())
}

fn check_void_return(ret_ty: &Ty, span: Span, return_type_span: Span) -> SemaResult<()> {
    if *ret_ty != Ty::Void {
        return Err(SemaError::ReturnTypeMismatch {
            expected: ret_ty.clone(),
            found: Ty::Void,
            span,
            return_type_span,
        });
    }
    Ok(())
}

fn check_condition(ty: &Ty, span: Span) -> SemaResult<()> {
    if *ty == Ty::Void {
        return Err(SemaError::VoidInValuePosition { span });
    }
    // TODO: check for boolean
    Ok(())
}
