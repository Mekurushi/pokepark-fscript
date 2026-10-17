use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ControlFlowError {
    BreakOutsideLoop { span: Span },
}

impl std::fmt::Display for ControlFlowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BreakOutsideLoop { .. } => formatter.write_str("`break` outside of a loop"),
        }
    }
}

impl std::error::Error for ControlFlowError {}

impl From<ControlFlowError> for Diagnostic {
    fn from(error: ControlFlowError) -> Self {
        let message = error.to_string();
        let ControlFlowError::BreakOutsideLoop { span } = error;
        Diagnostic::error(Stage::Semantic, message)
            .with_label(Label::primary(span, "not inside a loop"))
    }
}
