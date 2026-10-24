use super::builtin::{BuiltinArity, BuiltinContext};
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ResolveError {
    DuplicateDeclaration {
        name: String,
        duplicate_span: Span,
        original_span: Span,
    },
    UndeclaredName {
        name: String,
        reference_span: Span,
    },
    UnknownBuiltin {
        name: String,
        name_span: Span,
    },
    BuiltinArgumentCount {
        name: &'static str,
        expected: BuiltinArity,
        actual: usize,
        invocation_span: Span,
    },
    InvalidBuiltinContext {
        name: &'static str,
        context: BuiltinContext,
        invocation_span: Span,
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
            Self::UnknownBuiltin { name, .. } => write!(formatter, "unknown builtin `@{name}`"),
            Self::BuiltinArgumentCount {
                name,
                expected,
                actual,
                ..
            } => {
                write!(formatter, "builtin `@{name}` expects ")?;
                match expected {
                    BuiltinArity::Exact(count) => write!(
                        formatter,
                        "{count} argument{}",
                        if *count == 1 { "" } else { "s" }
                    )?,
                    BuiltinArity::AtLeast(count) => write!(
                        formatter,
                        "at least {count} argument{}",
                        if *count == 1 { "" } else { "s" }
                    )?,
                }
                write!(formatter, ", found {actual}")
            }
            Self::InvalidBuiltinContext { name, context, .. } => match context {
                BuiltinContext::Value => {
                    write!(formatter, "builtin `@{name}` cannot be used as a value")
                }
                BuiltinContext::Statement => {
                    write!(formatter, "builtin `@{name}` cannot be used as a statement")
                }
            },
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
            ResolveError::UnknownBuiltin { name_span, .. } => {
                diagnostic.with_label(Label::primary(name_span, "not a recognized builtin"))
            }
            ResolveError::BuiltinArgumentCount {
                invocation_span, ..
            } => {
                diagnostic.with_label(Label::primary(invocation_span, "wrong number of arguments"))
            }
            ResolveError::InvalidBuiltinContext {
                context,
                invocation_span,
                ..
            } => diagnostic.with_label(Label::primary(
                invocation_span,
                match context {
                    BuiltinContext::Value => "this builtin does not produce a value",
                    BuiltinContext::Statement => "this builtin is not valid as a statement",
                },
            )),
        }
    }
}
