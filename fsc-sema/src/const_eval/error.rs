use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ConstEvalError {
    DependencyCycle {
        reference_span: Span,
        constants: Vec<(String, Span)>,
    },
    NonConstantExpression {
        kind: &'static str,
        span: Span,
    },
    MissingConfigValue {
        name: String,
        span: Span,
    },
    IntegerOverflow {
        operation: &'static str,
        span: Span,
    },
    DivisionByZero {
        span: Span,
    },
    IndexOutOfBounds {
        index: i32,
        length: usize,
        span: Span,
    },
}

impl std::fmt::Display for ConstEvalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DependencyCycle { .. } => formatter.write_str("constant dependency cycle"),
            Self::NonConstantExpression { kind, .. } => {
                write!(formatter, "{kind} is not allowed in a constant expression")
            }
            Self::MissingConfigValue { name, .. } => {
                write!(formatter, "config `{name}` has no bound value")
            }
            Self::IntegerOverflow { operation, .. } => {
                write!(formatter, "integer overflow while evaluating `{operation}`")
            }
            Self::DivisionByZero { .. } => formatter.write_str("division by zero"),
            Self::IndexOutOfBounds { index, length, .. } => write!(
                formatter,
                "array index {index} is out of bounds for an array of length {length}"
            ),
        }
    }
}

impl std::error::Error for ConstEvalError {}

impl From<ConstEvalError> for Diagnostic {
    fn from(error: ConstEvalError) -> Self {
        let mut diagnostic = Diagnostic::error(Stage::Semantic, error.to_string());
        match error {
            ConstEvalError::DependencyCycle {
                reference_span,
                constants,
            } => {
                diagnostic = diagnostic.with_label(Label::primary(
                    reference_span,
                    "this reference closes the dependency cycle",
                ));
                for (name, span) in constants {
                    diagnostic = diagnostic.with_label(Label::secondary(
                        span,
                        format!("constant `{name}` participates in the cycle"),
                    ));
                }
                diagnostic
            }
            ConstEvalError::NonConstantExpression { span, .. } => {
                diagnostic.with_label(Label::primary(span, "cannot be evaluated at compile time"))
            }
            ConstEvalError::MissingConfigValue { span, .. } => diagnostic.with_label(
                Label::primary(span, "a value must be bound before constant evaluation"),
            ),
            ConstEvalError::IntegerOverflow { span, .. } => diagnostic.with_label(Label::primary(
                span,
                "the result does not fit in a signed 32-bit integer",
            )),
            ConstEvalError::DivisionByZero { span } => {
                diagnostic.with_label(Label::primary(span, "the divisor evaluates to zero"))
            }
            ConstEvalError::IndexOutOfBounds { span, .. } => {
                diagnostic.with_label(Label::primary(span, "out-of-bounds array access"))
            }
        }
    }
}
