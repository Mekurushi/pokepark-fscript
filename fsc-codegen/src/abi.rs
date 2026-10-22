use fsc_mir::{FunctionSignature, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PassMode {
    Ignore,
    Direct,
    Indirect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValueAbi {
    pub(crate) ty: Ty,
    pub(crate) mode: PassMode,
}

impl ValueAbi {
    pub(crate) fn from_type(ty: &Ty) -> Self {
        Self {
            ty: ty.clone(),
            mode: pass_mode(ty),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FunctionAbi {
    pub(crate) return_value: ValueAbi,
    pub(crate) parameters: Box<[ValueAbi]>,
}

impl FunctionAbi {
    pub(crate) fn from_signature(signature: &FunctionSignature) -> Self {
        Self {
            return_value: ValueAbi::from_type(&signature.return_type),
            parameters: signature
                .parameters
                .iter()
                .map(ValueAbi::from_type)
                .collect(),
        }
    }
}

pub(crate) const fn pass_mode(ty: &Ty) -> PassMode {
    match ty {
        Ty::Void => PassMode::Ignore,
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::Reference(_) => PassMode::Direct,
        Ty::Vec3 | Ty::Array { .. } => PassMode::Indirect,
    }
}
