use fsc_mir::Ty;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassMode {
    Direct,
    Indirect,
}

pub(crate) const fn pass_mode(ty: &Ty) -> Option<PassMode> {
    match ty {
        Ty::Void => None,
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str => Some(PassMode::Direct),
        Ty::Vec3 | Ty::Array { .. } => Some(PassMode::Indirect),
    }
}
