use super::ConstValue;
use super::body::BodyEvaluation;
use super::context::EvaluationContext;
use super::error::ConstEvalError;
use super::operation;
use crate::hir;
use fsc_diagnostics::{ErrorGuaranteed, Span};

#[derive(Clone, Copy)]
pub(super) enum LocalValues<'a> {
    Unavailable,
    Body {
        body: &'a hir::Body,
        evaluation: &'a BodyEvaluation,
    },
}

pub(super) struct ExpressionEvaluator<'context, 'analysis, 'diagnostics, 'locals> {
    context: &'context mut EvaluationContext<'analysis, 'diagnostics>,
    locals: LocalValues<'locals>,
}

impl<'context, 'analysis, 'diagnostics, 'locals>
    ExpressionEvaluator<'context, 'analysis, 'diagnostics, 'locals>
{
    pub(super) const fn new(
        context: &'context mut EvaluationContext<'analysis, 'diagnostics>,
        locals: LocalValues<'locals>,
    ) -> Self {
        Self { context, locals }
    }

    pub(super) fn evaluate(
        &mut self,
        expression: &hir::Expr,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        match &expression.kind {
            hir::ExprKind::Error(error) => Err(*error),
            hir::ExprKind::Int(value) => Ok(ConstValue::Int(*value)),
            hir::ExprKind::Float(value) => Ok(ConstValue::Float(*value)),
            hir::ExprKind::Bool(value) => Ok(ConstValue::Bool(*value)),
            hir::ExprKind::String(value) => Ok(ConstValue::String(value.clone())),
            hir::ExprKind::Array { elements } => elements
                .iter()
                .map(|element| self.evaluate(element))
                .collect::<Result<Vec<_>, _>>()
                .map(|elements| ConstValue::Array(elements.into_boxed_slice())),
            hir::ExprKind::Index { base, index } => {
                let base = self.evaluate(base)?;
                let index_span = index.span;
                let index = self.evaluate(index)?;
                let (ConstValue::Array(elements), ConstValue::Int(index)) = (base, index) else {
                    unreachable!("ill-typed array index reached constant evaluation")
                };
                let length = elements.len();
                usize::try_from(index)
                    .ok()
                    .and_then(|index| elements.get(index).cloned())
                    .ok_or_else(|| {
                        self.emit(ConstEvalError::IndexOutOfBounds {
                            index,
                            length,
                            span: index_span,
                        })
                    })
            }
            hir::ExprKind::ItemRef(item) => self.context.evaluate_item(*item, expression.span),
            hir::ExprKind::Local(local) => self.evaluate_local(*local, expression.span),
            hir::ExprKind::Member { .. } => Err(self.emit(ConstEvalError::NonConstantExpression {
                kind: "member access",
                span: expression.span,
            })),
            hir::ExprKind::Unary { op, expr } => {
                let operand = self.evaluate(expr)?;
                operation::unary(*op, operand, expression.span).map_err(|error| self.emit(error))
            }
            hir::ExprKind::Binary { .. } => self.evaluate_binary(expression),
            hir::ExprKind::Call { .. } => Err(self.emit(ConstEvalError::NonConstantExpression {
                kind: "function call",
                span: expression.span,
            })),
            hir::ExprKind::RawSyscall { .. } => {
                Err(self.emit(ConstEvalError::NonConstantExpression {
                    kind: "raw syscall",
                    span: expression.span,
                }))
            }
        }
    }

    fn evaluate_binary(&mut self, expression: &hir::Expr) -> Result<ConstValue, ErrorGuaranteed> {
        let hir::ExprKind::Binary { op, lhs, rhs } = &expression.kind else {
            unreachable!("non-binary expression reached binary constant evaluation");
        };
        let lhs = self.evaluate(lhs)?;
        match (op, &lhs) {
            (hir::BinaryOp::And, ConstValue::Bool(false)) => {
                return Ok(ConstValue::Bool(false));
            }
            (hir::BinaryOp::Or, ConstValue::Bool(true)) => {
                return Ok(ConstValue::Bool(true));
            }
            _ => {}
        }

        let rhs = self.evaluate(rhs)?;
        operation::binary(*op, lhs, rhs, expression.span).map_err(|error| self.emit(error))
    }

    fn evaluate_local(
        &mut self,
        local: hir::LocalId,
        span: Span,
    ) -> Result<ConstValue, ErrorGuaranteed> {
        let LocalValues::Body { body, evaluation } = self.locals else {
            return Err(self.emit(ConstEvalError::NonConstantExpression {
                kind: "local value",
                span,
            }));
        };

        match body.locals[local].kind {
            hir::LocalKind::Constant => {
                if let Some(value) = evaluation.constants.local(local) {
                    return Ok(value.clone());
                }
                if let Some(error) = evaluation.failures.get(local).and_then(|error| *error) {
                    return Err(error);
                }

                debug_assert!(false, "unevaluated local constant was referenced");
                Err(self.emit(ConstEvalError::NonConstantExpression {
                    kind: "unevaluated local constant",
                    span,
                }))
            }
            hir::LocalKind::Parameter { .. } | hir::LocalKind::Variable => {
                Err(self.emit(ConstEvalError::NonConstantExpression {
                    kind: "runtime local value",
                    span,
                }))
            }
        }
    }

    fn emit(&mut self, error: ConstEvalError) -> ErrorGuaranteed {
        self.context.emit_error(error)
    }
}
