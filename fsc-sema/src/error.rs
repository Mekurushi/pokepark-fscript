use crate::types::Ty;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq)]
pub enum SemaError {
    UndeclaredName {
        name: String,
        reference_span: Span,
    },

    DuplicateDeclaration {
        name: String,
        duplicate_span: Span,
        original_span: Span,
    },

    TypeMismatch {
        expected: Ty,
        found: Ty,
        span: Span,
        expected_span: Option<Span>,
    },

    ReturnTypeMismatch {
        expected: Ty,
        found: Ty,
        span: Span,
        return_type_span: Span,
    },

    VoidInValuePosition {
        span: Span,
    },
    NotCallable {
        name: String,
        callee_span: Span,
        declaration_span: Span,
    },
    ArgumentCountMismatch {
        name: String,
        expected: usize,
        found: usize,
        call_span: Span,
        declaration_span: Span,
    },
    InvalidConstantType {
        ty: Ty,
        type_span: Span,
    },
    InvalidConfigType {
        ty: Ty,
        type_span: Span,
    },
    InvalidParameterType {
        ty: Ty,
        type_span: Span,
    },
    InvalidReturnType {
        ty: Ty,
        type_span: Span,
    },
    InvalidLocalType {
        ty: Ty,
        type_span: Span,
    },
    InvalidAssignmentTarget {
        target_span: Span,
    },
    ConstantInitializerMustBeLiteral {
        initializer_span: Span,
    },
    AssignmentToConstant {
        name: String,
        assignment_span: Span,
        declaration_span: Span,
    },
    AssignmentToConfig {
        name: String,
        assignment_span: Span,
        declaration_span: Span,
    },
    MissingConfigValue {
        name: String,
        declaration_span: Span,
    },
    NotAValue {
        name: String,
        reference_span: Span,
        declaration_span: Span,
    },
    MissingLocalIdentity {
        name: String,
        reference_span: Span,
    },
    MissingExpressionType {
        span: Span,
    },
    MissingFunctionSignature {
        name: String,
        declaration_span: Span,
    },
    UnsupportedValueType {
        ty: Ty,
        span: Span,
    },
    InvalidMember {
        ty: Ty,
        member: String,
        member_span: Span,
    },
}

impl std::fmt::Display for SemaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndeclaredName { name, .. } => {
                write!(f, "undeclared name `{name}`")
            }
            Self::DuplicateDeclaration { name, .. } => {
                write!(f, "`{name}` is already declared in this scope")
            }
            Self::TypeMismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "type mismatch: expected `{expected:?}`, found `{found:?}`"
                )
            }
            Self::ReturnTypeMismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "return type mismatch: \
                     function declares `{expected:?}` but expression has type `{found:?}`"
                )
            }
            Self::VoidInValuePosition { .. } => {
                write!(f, "expression has type `void` where a value is required")
            }
            Self::NotCallable { name, .. } => {
                write!(f, "function is not callable {name}")
            }
            Self::ArgumentCountMismatch {
                name,
                expected,
                found,
                ..
            } => write!(
                f,
                "function `{name}` expects {expected} arguments, but {found} were provided"
            ),
            Self::InvalidConstantType { ty, .. } => {
                write!(f, "constants cannot have type `{ty:?}`")
            }
            Self::InvalidConfigType { ty, .. } => {
                write!(f, "configs cannot have type `{ty:?}`")
            }
            Self::InvalidParameterType { ty, .. } => {
                write!(f, "parameters cannot have type `{ty:?}`")
            }
            Self::InvalidReturnType { ty, .. } => {
                write!(f, "functions cannot return type `{ty:?}`")
            }
            Self::InvalidLocalType { ty, .. } => {
                write!(f, "local variables cannot have type `{ty:?}`")
            }
            Self::InvalidAssignmentTarget { .. } => {
                write!(f, "assignment target is not a variable")
            }
            Self::ConstantInitializerMustBeLiteral { .. } => {
                write!(f, "constant initializer must be a literal")
            }
            Self::AssignmentToConstant { name, .. } => {
                write!(f, "cannot assign to constant `{name}`")
            }
            Self::AssignmentToConfig { name, .. } => {
                write!(f, "cannot assign to config `{name}`")
            }
            Self::MissingConfigValue { name, .. } => {
                write!(f, "missing bound value for config `{name}`")
            }
            Self::NotAValue { name, .. } => {
                write!(f, "`{name}` is not a value")
            }
            Self::MissingLocalIdentity { name, .. } => {
                write!(f, "local `{name}` has no logical identity")
            }
            Self::MissingExpressionType { .. } => {
                write!(
                    f,
                    "internal compiler error: checked expression has no semantic type"
                )
            }
            Self::MissingFunctionSignature { name, .. } => {
                write!(
                    f,
                    "internal compiler error: `{name}` has no semantic function signature"
                )
            }
            Self::UnsupportedValueType { ty, .. } => {
                write!(
                    f,
                    "values of type `{ty:?}` are not supported in this context"
                )
            }
            Self::InvalidMember { ty, member, .. } => {
                write!(f, "type `{ty:?}` has no member `{member}`")
            }
        }
    }
}

impl From<SemaError> for Diagnostic {
    fn from(error: SemaError) -> Self {
        let message = error.to_string();
        let diagnostic = Diagnostic::error(Stage::Semantic, message);
        match error {
            SemaError::UndeclaredName { reference_span, .. } => {
                diagnostic.with_label(Label::primary(reference_span, "not found in this scope"))
            }
            SemaError::DuplicateDeclaration {
                duplicate_span,
                original_span,
                ..
            } => diagnostic
                .with_label(Label::primary(duplicate_span, "redeclared here"))
                .with_label(Label::secondary(original_span, "first declared here")),
            SemaError::TypeMismatch {
                expected,
                found,
                span,
                expected_span,
            } => {
                let diagnostic =
                    diagnostic.with_label(Label::primary(span, format!("has type `{found:?}`")));
                if let Some(expected_span) = expected_span {
                    diagnostic.with_label(Label::secondary(
                        expected_span,
                        format!("expected `{expected:?}` because of this"),
                    ))
                } else {
                    diagnostic
                }
            }
            SemaError::ReturnTypeMismatch {
                expected,
                found,
                span,
                return_type_span,
            } => diagnostic
                .with_label(Label::primary(span, format!("returns `{found:?}`")))
                .with_label(Label::secondary(
                    return_type_span,
                    format!("function declares `{expected:?}` here"),
                )),
            SemaError::VoidInValuePosition { span } => {
                diagnostic.with_label(Label::primary(span, "has type `Void`"))
            }
            SemaError::NotCallable {
                callee_span,
                declaration_span,
                ..
            } => diagnostic
                .with_label(Label::primary(callee_span, "called here"))
                .with_label(Label::secondary(
                    declaration_span,
                    "this value is declared here",
                )),
            SemaError::ArgumentCountMismatch {
                expected,
                found,
                call_span,
                declaration_span,
                ..
            } => diagnostic
                .with_label(Label::primary(
                    call_span,
                    format!("expected {expected} arguments, found {found}"),
                ))
                .with_label(Label::secondary(declaration_span, "function declared here")),
            SemaError::InvalidConstantType { ty, type_span } => diagnostic.with_label(
                Label::primary(type_span, format!("`{ty:?}` cannot be used for a constant")),
            ),
            SemaError::InvalidConfigType { ty, type_span } => diagnostic.with_label(
                Label::primary(type_span, format!("`{ty:?}` cannot be used for a config")),
            ),
            SemaError::InvalidParameterType { ty, type_span } => {
                diagnostic.with_label(Label::primary(
                    type_span,
                    format!("`{ty:?}` cannot be used for a parameter"),
                ))
            }
            SemaError::InvalidReturnType { ty, type_span } => {
                diagnostic.with_label(Label::primary(
                    type_span,
                    format!("`{ty:?}` cannot be returned by a function"),
                ))
            }
            SemaError::InvalidLocalType { ty, type_span } => diagnostic.with_label(Label::primary(
                type_span,
                format!("`{ty:?}` cannot be used for a local variable"),
            )),
            SemaError::InvalidAssignmentTarget { target_span } => diagnostic.with_label(
                Label::primary(target_span, "only variables can be assigned to"),
            ),
            SemaError::ConstantInitializerMustBeLiteral { initializer_span } => {
                diagnostic.with_label(Label::primary(initializer_span, "not a literal expression"))
            }
            SemaError::AssignmentToConstant {
                assignment_span,
                declaration_span,
                ..
            } => diagnostic
                .with_label(Label::primary(assignment_span, "assigned here"))
                .with_label(Label::secondary(declaration_span, "constant declared here")),
            SemaError::AssignmentToConfig {
                assignment_span,
                declaration_span,
                ..
            } => diagnostic
                .with_label(Label::primary(assignment_span, "assigned here"))
                .with_label(Label::secondary(declaration_span, "config declared here")),
            SemaError::MissingConfigValue {
                declaration_span, ..
            } => diagnostic.with_label(Label::primary(
                declaration_span,
                "this config was not bound before lowering",
            )),
            SemaError::NotAValue {
                reference_span,
                declaration_span,
                ..
            } => diagnostic
                .with_label(Label::primary(reference_span, "used as a value here"))
                .with_label(Label::secondary(declaration_span, "function declared here")),
            SemaError::MissingLocalIdentity { reference_span, .. } => diagnostic.with_label(
                Label::primary(reference_span, "this local has no semantic identity"),
            ),
            SemaError::MissingExpressionType { span } => diagnostic.with_label(Label::primary(
                span,
                "semantic type information is missing for this expression",
            )),
            SemaError::MissingFunctionSignature {
                declaration_span, ..
            } => diagnostic.with_label(Label::primary(
                declaration_span,
                "resolved function identity does not refer to a function signature",
            )),
            SemaError::UnsupportedValueType { ty, span } => diagnostic.with_label(Label::primary(
                span,
                format!("whole `{ty:?}` values are not supported yet"),
            )),
            SemaError::InvalidMember {
                ty,
                member,
                member_span,
            } => diagnostic.with_label(Label::primary(
                member_span,
                format!("`{member}` is not a member of `{ty:?}`"),
            )),
        }
    }
}
pub type SemaResult<T> = Result<T, SemaError>;
