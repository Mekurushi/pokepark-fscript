use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    DuplicateDeclaration {
        name: String,
        duplicate_span: Span,
        original_span: Span,
    },
    UndeclaredName {
        name: String,
        reference_span: Span,
    },
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateDeclaration { name, .. } => {
                write!(formatter, "`{name}` is already declared in this scope")
            }
            Self::UndeclaredName { name, .. } => {
                write!(formatter, "undeclared name `{name}`")
            }
        }
    }
}

impl std::error::Error for ResolveError {}

impl From<ResolveError> for Diagnostic {
    fn from(error: ResolveError) -> Self {
        let diagnostic = Diagnostic::error(Stage::Semantic, error.to_string());
        match error {
            ResolveError::DuplicateDeclaration {
                duplicate_span,
                original_span,
                ..
            } => diagnostic
                .with_label(Label::primary(duplicate_span, "redeclared here"))
                .with_label(Label::secondary(original_span, "first declared here")),
            ResolveError::UndeclaredName { reference_span, .. } => {
                diagnostic.with_label(Label::primary(reference_span, "not found in this scope"))
            }
        }
    }
}
