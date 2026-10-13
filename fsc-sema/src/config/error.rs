use super::ConfigType;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum BindError {
    MissingValue {
        name: String,
        declaration_span: Span,
    },
    UnknownConfig {
        name: String,
    },
    TypeMismatch {
        name: String,
        expected: ConfigType,
        actual: ConfigType,
        declaration_span: Span,
    },
}

impl std::fmt::Display for BindError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingValue { name, .. } => {
                write!(formatter, "missing value for config `{name}`")
            }
            Self::UnknownConfig { name } => write!(formatter, "unknown config `{name}`"),
            Self::TypeMismatch {
                name,
                expected,
                actual,
                ..
            } => write!(
                formatter,
                "config `{name}` expects `{expected}`, but received `{actual}`"
            ),
        }
    }
}

impl std::error::Error for BindError {}
impl From<BindError> for Diagnostic {
    fn from(error: BindError) -> Self {
        let diagnostic = Diagnostic::error(Stage::Semantic, error.to_string());
        match error {
            BindError::MissingValue {
                declaration_span, ..
            } => diagnostic.with_label(Label::primary(
                declaration_span,
                "a value is required for this config",
            )),
            BindError::UnknownConfig { .. } => diagnostic,
            BindError::TypeMismatch {
                expected,
                declaration_span,
                ..
            } => diagnostic.with_label(Label::primary(
                declaration_span,
                format!("declared as `{expected}` here"),
            )),
        }
    }
}
