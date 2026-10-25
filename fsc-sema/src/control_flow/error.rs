use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ControlFlowError {
    BreakOutsideLoop { span: Span },
    ContinueOutsideLoop { span: Span },
}

impl std::fmt::Display for ControlFlowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BreakOutsideLoop { .. } => formatter.write_str("`break` outside of a loop"),
            Self::ContinueOutsideLoop { .. } => {
                formatter.write_str("`continue` outside of a loop")
            }
        }
    }
}

impl std::error::Error for ControlFlowError {}

impl From<ControlFlowError> for Diagnostic {
    fn from(error: ControlFlowError) -> Self {
        let message = error.to_string();
        let span = match error {
            ControlFlowError::BreakOutsideLoop { span }
            | ControlFlowError::ContinueOutsideLoop { span } => span,
        };
        Diagnostic::error(Stage::Semantic, message)
            .with_label(Label::primary(span, "not inside a loop"))
    }
}
