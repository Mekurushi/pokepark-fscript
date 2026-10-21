use fsc_mir::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassMode {
    Direct,
    Indirect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParameterAbi {
    pub(crate) ty: Ty,
    pub(crate) mode: PassMode,
}

impl ParameterAbi {
    pub(crate) fn from_type(ty: &Ty) -> Option<Self> {
        Some(Self {
            ty: ty.clone(),
            mode: pass_mode(ty)?,
        })
    }
}

pub(crate) const fn pass_mode(ty: &Ty) -> Option<PassMode> {
    match ty {
        Ty::Void => None,
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Reference(_) => Some(PassMode::Direct),
        Ty::Vec3 | Ty::Array { .. } => Some(PassMode::Indirect),
    }
}
