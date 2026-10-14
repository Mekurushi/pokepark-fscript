use crate::types::Ty;

// intention: separating cleanly language design rules and HIR traversal

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Compatibility {
    Exact,
    Incompatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TypeContext {
    Parameter,
    Local,
    Constant,
    Config,
    Return,
}

impl TypeContext {
    pub(super) const fn description(self) -> &'static str {
        match self {
            Self::Parameter => "parameter",
            Self::Local => "local variable",
            Self::Constant => "constant",
            Self::Config => "config value",
            Self::Return => "function return",
        }
    }
}

pub(super) fn compatibility(actual: &Ty, expected: &Ty) -> Compatibility {
    if actual == expected {
        Compatibility::Exact
    } else {
        Compatibility::Incompatible
    }
}

pub(super) fn type_allowed(ty: &Ty, context: TypeContext) -> bool {
    match context {
        TypeContext::Parameter | TypeContext::Local => *ty != Ty::Void,
        // TODO: add vec3 constants with proper end-to-end support (aggregates etc.)
        TypeContext::Constant => *ty != Ty::Void && *ty != Ty::Vec3,
        TypeContext::Config => matches!(ty, Ty::Int | Ty::Float | Ty::Bool | Ty::Str),
        TypeContext::Return => true,
    }
}

pub(super) fn supports_arithmetic(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float)
}

pub(super) fn supports_negation(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float)
}

pub(super) fn supports_ordering(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float)
}

pub(super) fn supports_equality(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float | Ty::Bool)
}
