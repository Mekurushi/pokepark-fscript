#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Int,
    Float,
    Vec3,
    Array { element: Box<Ty>, length: usize },
    Void,
    Bool,
    Str,
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
