use super::body::BodyLowerer;
use crate::config::ConfigValue;
use crate::const_eval::ConstValue;
use crate::typeck::ExprType;
use crate::types::Ty;
use crate::{hir, thir};

impl BodyLowerer<'_, '_> {
    pub(super) fn lower_expression(&mut self, expression: &hir::Expr) -> thir::ExprId {
        let kind = match &expression.kind {
            hir::ExprKind::Error(_) => unreachable!("error expression reached THIR lowering"),
            hir::ExprKind::Int(value) => thir::ExprKind::Constant(thir::ConstValue::Int(*value)),
            hir::ExprKind::Float(value) => {
                thir::ExprKind::Constant(thir::ConstValue::Float(*value))
            }
            hir::ExprKind::Bool(value) => thir::ExprKind::Constant(thir::ConstValue::Bool(*value)),
            hir::ExprKind::String(value) => {
                thir::ExprKind::Constant(thir::ConstValue::String(value.clone()))
            }
            hir::ExprKind::Local(local) => self.lower_local(*local),
            hir::ExprKind::ItemRef(item) => self.lower_item(*item),
            hir::ExprKind::Member { base, .. } => {
                let base = self.lower_expression(base);
                let Some(field) = self.context.typeck.field_index(expression.id) else {
                    unreachable!("unresolved field reached THIR lowering")
                };
                thir::ExprKind::Field { base, field }
            }
            hir::ExprKind::Unary { op, expr } => thir::ExprKind::Unary {
                op: lower_unary_operator(*op),
                operand: self.lower_expression(expr),
            },
            hir::ExprKind::Binary { op, lhs, rhs } => thir::ExprKind::Binary {
                op: lower_binary_operator(*op),
                lhs: self.lower_expression(lhs),
                rhs: self.lower_expression(rhs),
            },
            hir::ExprKind::Call { callee, arguments } => {
                let Some(ExprType::Function(callee)) =
                    self.context.typeck.expression_type(callee.id)
                else {
                    unreachable!("unresolved callee reached THIR lowering")
                };
                let Some(callee) = self.context.functions.function(*callee) else {
                    unreachable!("non-function callee reached THIR lowering")
                };
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect();
                thir::ExprKind::Call { callee, arguments }
            }
            hir::ExprKind::RawSyscall { arguments } => {
                // destructuring the selector arguments out; again an implicit invariant, but tbh
                // no clue if there is any better way; it should be good enough since syscalls
                // are hopefully a temporary construct and/or escape solution
                let [_, _, arguments @ ..] = arguments.as_slice() else {
                    unreachable!("raw syscall without selectors reached THIR lowering")
                };
                let Some(selectors) = self.context.raw_syscalls.selectors(expression.id) else {
                    unreachable!("unresolved raw syscall selectors reached THIR lowering")
                };
                thir::ExprKind::RawSyscall {
                    page: thir::RawSyscallPage::new(selectors.page),
                    function: thir::RawSyscallFunction::new(selectors.function),
                    arguments: arguments
                        .iter()
                        .map(|argument| self.lower_expression(argument))
                        .collect(),
                }
            }
        };
        let ty = self.expression_type(expression);
        self.expressions.push(thir::Expr {
            kind,
            ty,
            source: thir::SourceInfo {
                span: expression.span,
            },
        })
    }

    fn lower_local(&self, local: hir::LocalId) -> thir::ExprKind {
        match self.body.locals[local].kind {
            hir::LocalKind::Parameter { .. } | hir::LocalKind::Variable => {
                thir::ExprKind::Local(Self::mapped_local(&self.local_mapping, local))
            }
            hir::LocalKind::Constant => {
                let Some(value) = self.context.constants.local(self.owner, local) else {
                    unreachable!("unevaluated local constant reached THIR lowering")
                };
                thir::ExprKind::Constant(lower_constant(value))
            }
        }
    }

    fn lower_item(&self, item: hir::ItemId) -> thir::ExprKind {
        match &self.context.hir.items[item].kind {
            hir::ItemKind::Constant(_) => {
                let Some(value) = self.context.constants.item(item) else {
                    unreachable!("unevaluated constant reached THIR lowering")
                };
                thir::ExprKind::Constant(lower_constant(value))
            }
            hir::ItemKind::Config(_) => {
                let Some(value) = self.context.configs.value(item) else {
                    unreachable!("unbound config reached THIR lowering")
                };
                thir::ExprKind::Constant(lower_config(value))
            }
            hir::ItemKind::Function(_) => unreachable!("function item reached THIR value lowering"),
        }
    }

    fn expression_type(&self, expression: &hir::Expr) -> Ty {
        let Some(ExprType::Value(ty)) = self.context.typeck.expression_type(expression.id) else {
            unreachable!("untyped value expression reached THIR lowering")
        };
        ty.clone()
    }
}

fn lower_constant(value: &ConstValue) -> thir::ConstValue {
    match value {
        ConstValue::Int(value) => thir::ConstValue::Int(*value),
        ConstValue::Float(value) => thir::ConstValue::Float(*value),
        ConstValue::Bool(value) => thir::ConstValue::Bool(*value),
        ConstValue::String(value) => thir::ConstValue::String(value.clone()),
    }
}

fn lower_config(value: &ConfigValue) -> thir::ConstValue {
    match value {
        ConfigValue::Int(value) => thir::ConstValue::Int(*value),
        ConfigValue::Float(value) => thir::ConstValue::Float(*value),
        ConfigValue::Bool(value) => thir::ConstValue::Bool(*value),
        ConfigValue::String(value) => thir::ConstValue::String(value.clone()),
    }
}

const fn lower_unary_operator(operator: hir::UnaryOp) -> thir::UnaryOp {
    match operator {
        hir::UnaryOp::Not => thir::UnaryOp::Not,
        hir::UnaryOp::Neg => thir::UnaryOp::Negate,
    }
}

const fn lower_binary_operator(operator: hir::BinaryOp) -> thir::BinaryOp {
    match operator {
        hir::BinaryOp::Add => thir::BinaryOp::Add,
        hir::BinaryOp::Sub => thir::BinaryOp::Subtract,
        hir::BinaryOp::Mul => thir::BinaryOp::Multiply,
        hir::BinaryOp::Div => thir::BinaryOp::Divide,
        hir::BinaryOp::Eq => thir::BinaryOp::Equal,
        hir::BinaryOp::NotEq => thir::BinaryOp::NotEqual,
        hir::BinaryOp::Less => thir::BinaryOp::Less,
        hir::BinaryOp::Greater => thir::BinaryOp::Greater,
        hir::BinaryOp::LessEq => thir::BinaryOp::LessEqual,
        hir::BinaryOp::GreaterEq => thir::BinaryOp::GreaterEqual,
        hir::BinaryOp::And => thir::BinaryOp::LogicalAnd,
        hir::BinaryOp::Or => thir::BinaryOp::LogicalOr,
    }
}
