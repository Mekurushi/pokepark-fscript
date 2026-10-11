use crate::types::Ty;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub enum BindError {
    MissingValue {
        name: String,
        declaration_span: Span,
    },
    UnknownConfig {
        name: String,
    },
    TypeMismatch {
        name: String,
        expected: Ty,
        found: Ty,
        declaration_span: Span,
    },
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingValue { name, .. } => write!(f, "missing value for config `{name}`"),
            Self::UnknownConfig { name } => write!(f, "unknown config `{name}`"),
            Self::TypeMismatch {
                name,
                expected,
                found,
                ..
            } => write!(
                f,
                "config `{name}` expects `{expected:?}`, but received `{found:?}`"
            ),
        }
    }
}

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
                format!("declared as `{expected:?}` here"),
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BindErrors {
    errors: Vec<BindError>,
}

impl BindErrors {
    pub(crate) fn new(errors: Vec<BindError>) -> Self {
        Self { errors }
    }

    #[must_use]
    pub fn errors(&self) -> &[BindError] {
        &self.errors
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.errors.into_iter().map(Diagnostic::from).collect()
    }
}

impl std::fmt::Display for BindErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} configuration binding error(s)", self.errors.len())
    }
}

impl std::error::Error for BindErrors {}
