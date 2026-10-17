use crate::hir;
use crate::types::Ty;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum TypeckError {
    InvalidDeclaredType {
        ty: Ty,
        span: Span,
        declaration: &'static str,
    },
    UnsupportedConfigType {
        ty: Ty,
        span: Span,
    },
    TypeMismatch {
        expected: Ty,
        actual: Ty,
        span: Span,
    },
    ExpectedValue {
        span: Span,
    },
    NotCallable {
        span: Span,
    },
    ArgumentCount {
        expected: usize,
        actual: usize,
        span: Span,
    },
    MissingRawSyscallSelectors {
        actual: usize,
        span: Span,
    },
    InvalidUnaryOperand {
        op: hir::UnaryOp,
        actual: Ty,
        span: Span,
    },
    InvalidBinaryOperands {
        op: hir::BinaryOp,
        lhs: Ty,
        rhs: Ty,
        span: Span,
    },
    UnknownMember {
        member: String,
        span: Span,
    },
    InvalidMemberBase {
        actual: Ty,
        span: Span,
    },
    NotAssignable {
        span: Span,
    },
    ImmutableItem {
        span: Span,
    },
    ImmutableLocalConstant {
        span: Span,
    },
}

impl std::fmt::Display for TypeckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeclaredType {
                ty, declaration, ..
            } => write!(
                formatter,
                "{declaration} cannot have type `{}`",
                TypeName(ty)
            ),
            Self::UnsupportedConfigType { ty, .. } => {
                write!(
                    formatter,
                    "config values cannot have type `{}`",
                    TypeName(ty)
                )
            }
            Self::TypeMismatch {
                expected, actual, ..
            } => write!(
                formatter,
                "expected `{}`, found `{}`",
                TypeName(expected),
                TypeName(actual)
            ),
            Self::ExpectedValue { .. } => formatter.write_str("expected a value"),
            Self::NotCallable { .. } => formatter.write_str("expression is not callable"),
            Self::ArgumentCount {
                expected, actual, ..
            } => write!(
                formatter,
                "expected {expected} argument{}, found {actual}",
                if *expected == 1 { "" } else { "s" }
            ),
            Self::MissingRawSyscallSelectors { actual, .. } => write!(
                formatter,
                "raw syscall requires page and function selectors; expected at least 2 arguments, found {actual}"
            ),
            Self::InvalidUnaryOperand { op, actual, .. } => write!(
                formatter,
                "operator `{}` cannot be applied to `{}`",
                UnaryOpName(*op),
                TypeName(actual)
            ),
            Self::InvalidBinaryOperands { op, lhs, rhs, .. } => write!(
                formatter,
                "operator `{}` cannot be applied to `{}` and `{}`",
                BinaryOpName(*op),
                TypeName(lhs),
                TypeName(rhs)
            ),
            Self::UnknownMember { member, .. } => write!(formatter, "unknown member `{member}`"),
            Self::InvalidMemberBase { actual, .. } => write!(
                formatter,
                "cannot access a member of `{}`",
                TypeName(actual)
            ),
            Self::NotAssignable { .. } => formatter.write_str("expression is not assignable"),
            Self::ImmutableItem { .. } => formatter.write_str("cannot assign to an immutable item"),
            Self::ImmutableLocalConstant { .. } => {
                formatter.write_str("cannot assign to a local constant")
            }
        }
    }
}

impl std::error::Error for TypeckError {}

impl From<TypeckError> for Diagnostic {
    fn from(error: TypeckError) -> Self {
        let diagnostic = Diagnostic::error(Stage::Semantic, error.to_string());
        let (span, label) = match &error {
            TypeckError::InvalidDeclaredType { span, .. }
            | TypeckError::UnsupportedConfigType { span, .. } => (*span, "invalid type"),
            TypeckError::TypeMismatch { span, .. } => (*span, "has the wrong type"),
            TypeckError::ExpectedValue { span } => (*span, "this is not a value"),
            TypeckError::NotCallable { span } => (*span, "this cannot be called"),
            TypeckError::ArgumentCount { span, .. } => (*span, "wrong number of arguments"),
            TypeckError::MissingRawSyscallSelectors { span, .. } => {
                (*span, "missing raw syscall selector")
            }
            TypeckError::InvalidUnaryOperand { span, .. }
            | TypeckError::InvalidBinaryOperands { span, .. } => (*span, "invalid operand type"),
            TypeckError::UnknownMember { span, .. } => (*span, "unknown member"),
            TypeckError::InvalidMemberBase { span, .. } => (*span, "has no members"),
            TypeckError::NotAssignable { span } => (*span, "not an assignment target"),
            TypeckError::ImmutableItem { span } => (*span, "declared immutable"),
            TypeckError::ImmutableLocalConstant { span } => (*span, "declared constant"),
        };
        diagnostic.with_label(Label::primary(span, label))
    }
}

struct TypeName<'a>(&'a Ty);

impl std::fmt::Display for TypeName<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Ty::Int => formatter.write_str("int"),
            Ty::Float => formatter.write_str("float"),
            Ty::Vec3 => formatter.write_str("vec3"),
            Ty::Array { element, length } => {
                write!(formatter, "[{}; {length}]", TypeName(element))
            }
            Ty::Void => formatter.write_str("void"),
            Ty::Bool => formatter.write_str("bool"),
            Ty::Str => formatter.write_str("string"),
        }
    }
}

struct UnaryOpName(hir::UnaryOp);

impl std::fmt::Display for UnaryOpName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.0 {
            hir::UnaryOp::Not => "!",
            hir::UnaryOp::Neg => "-",
        })
    }
}

struct BinaryOpName(hir::BinaryOp);

impl std::fmt::Display for BinaryOpName {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self.0 {
            hir::BinaryOp::Add => "+",
            hir::BinaryOp::Sub => "-",
            hir::BinaryOp::Mul => "*",
            hir::BinaryOp::Div => "/",
            hir::BinaryOp::Eq => "==",
            hir::BinaryOp::NotEq => "!=",
            hir::BinaryOp::Less => "<",
            hir::BinaryOp::Greater => ">",
            hir::BinaryOp::LessEq => "<=",
            hir::BinaryOp::GreaterEq => ">=",
            hir::BinaryOp::And => "&&",
            hir::BinaryOp::Or => "||",
        })
    }
}
