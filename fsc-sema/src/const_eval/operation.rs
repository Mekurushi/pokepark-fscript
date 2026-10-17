use super::ConstValue;
use super::error::ConstEvalError;
use crate::hir;
use fsc_diagnostics::Span;

pub(super) fn unary(
    operator: hir::UnaryOp,
    operand: ConstValue,
    span: Span,
) -> Result<ConstValue, ConstEvalError> {
    match (operator, operand) {
        (hir::UnaryOp::Not, ConstValue::Bool(value)) => Ok(ConstValue::Bool(!value)),
        (hir::UnaryOp::Neg, ConstValue::Int(value)) => value
            .checked_neg()
            .map(ConstValue::Int)
            .ok_or(ConstEvalError::IntegerOverflow {
                operation: "unary -",
                span,
            }),
        (hir::UnaryOp::Neg, ConstValue::Float(value)) => Ok(ConstValue::Float(-value)),
        _ => unreachable!("invalid unary operation reached constant evaluation"),
    }
}

// TODO: evaluating uses now rusts interpretation of e.g. float comparisons; I'm not sure if that
// really influences it in a bad way, so need to check that out and maybe think of a better solution
#[allow(clippy::float_cmp)]
pub(super) fn binary(
    operator: hir::BinaryOp,
    lhs: ConstValue,
    rhs: ConstValue,
    span: Span,
) -> Result<ConstValue, ConstEvalError> {
    use hir::BinaryOp::{Add, And, Div, Eq, Greater, GreaterEq, Less, LessEq, Mul, NotEq, Or, Sub};

    match (operator, lhs, rhs) {
        (Add, ConstValue::Int(lhs), ConstValue::Int(rhs)) => lhs
            .checked_add(rhs)
            .map(ConstValue::Int)
            .ok_or(ConstEvalError::IntegerOverflow {
                operation: "+",
                span,
            }),
        (Sub, ConstValue::Int(lhs), ConstValue::Int(rhs)) => lhs
            .checked_sub(rhs)
            .map(ConstValue::Int)
            .ok_or(ConstEvalError::IntegerOverflow {
                operation: "-",
                span,
            }),
        (Mul, ConstValue::Int(lhs), ConstValue::Int(rhs)) => lhs
            .checked_mul(rhs)
            .map(ConstValue::Int)
            .ok_or(ConstEvalError::IntegerOverflow {
                operation: "*",
                span,
            }),
        (Div, ConstValue::Int(_), ConstValue::Int(0)) => {
            Err(ConstEvalError::DivisionByZero { span })
        }
        (Div, ConstValue::Int(lhs), ConstValue::Int(rhs)) => lhs
            .checked_div(rhs)
            .map(ConstValue::Int)
            .ok_or(ConstEvalError::IntegerOverflow {
                operation: "/",
                span,
            }),

        (Add, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Float(lhs + rhs)),
        (Sub, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Float(lhs - rhs)),
        (Mul, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Float(lhs * rhs)),
        (Div, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Float(lhs / rhs)),

        (Eq, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs == rhs)),
        (NotEq, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs != rhs)),
        (Less, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs < rhs)),
        (Greater, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs > rhs)),
        (LessEq, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs <= rhs)),
        (GreaterEq, ConstValue::Int(lhs), ConstValue::Int(rhs)) => Ok(ConstValue::Bool(lhs >= rhs)),

        (Eq, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Bool(lhs == rhs)),
        (NotEq, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Bool(lhs != rhs)),
        (Less, ConstValue::Float(lhs), ConstValue::Float(rhs)) => Ok(ConstValue::Bool(lhs < rhs)),
        (Greater, ConstValue::Float(lhs), ConstValue::Float(rhs)) => {
            Ok(ConstValue::Bool(lhs > rhs))
        }
        (LessEq, ConstValue::Float(lhs), ConstValue::Float(rhs)) => {
            Ok(ConstValue::Bool(lhs <= rhs))
        }
        (GreaterEq, ConstValue::Float(lhs), ConstValue::Float(rhs)) => {
            Ok(ConstValue::Bool(lhs >= rhs))
        }

        (Eq, ConstValue::Bool(lhs), ConstValue::Bool(rhs)) => Ok(ConstValue::Bool(lhs == rhs)),
        (NotEq, ConstValue::Bool(lhs), ConstValue::Bool(rhs)) => Ok(ConstValue::Bool(lhs != rhs)),
        (And, ConstValue::Bool(lhs), ConstValue::Bool(rhs)) => Ok(ConstValue::Bool(lhs && rhs)),
        (Or, ConstValue::Bool(lhs), ConstValue::Bool(rhs)) => Ok(ConstValue::Bool(lhs || rhs)),

        _ => unreachable!("invalid binary operation reached constant evaluation"),
    }
}
