use super::SelectorKind;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) struct SelectorOutOfRange {
    pub(super) selector: SelectorKind,
    pub(super) value: i32,
    pub(super) span: Span,
}

impl std::fmt::Display for SelectorOutOfRange {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "raw syscall {} selector `{}` is outside the supported range 0..={}",
            self.selector.name(),
            self.value,
            self.selector.maximum()
        )
    }
}

impl std::error::Error for SelectorOutOfRange {}

impl From<SelectorOutOfRange> for Diagnostic {
    fn from(error: SelectorOutOfRange) -> Self {
        let message = error.to_string();
        Diagnostic::error(Stage::Semantic, message).with_label(Label::primary(
            error.span,
            format!(
                "{} selector must fit in 0..={}",
                error.selector.name(),
                error.selector.maximum()
            ),
        ))
    }
}
