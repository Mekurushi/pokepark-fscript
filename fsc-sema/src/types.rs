#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    Vec3,
    Void,
    Bool,
    Str,
}

impl Ty {
    #[must_use]
    pub const fn slot_width(&self) -> u16 {
        match self {
            Self::Void => 0,
            Self::Vec3 => 3,
            Self::Int | Self::Float | Self::Bool | Self::Str => 1,
        }
    }
}

impl From<&fsc_parse::ast::Ty> for Ty {
    fn from(ty: &fsc_parse::ast::Ty) -> Self {
        match ty {
            fsc_parse::ast::Ty::Int => Self::Int,
            fsc_parse::ast::Ty::Float => Self::Float,
            fsc_parse::ast::Ty::Vec3 => Self::Vec3,
            fsc_parse::ast::Ty::Void => Self::Void,
            fsc_parse::ast::Ty::Bool => Self::Bool,
            fsc_parse::ast::Ty::Str => Self::Str,
        }
    }
}
