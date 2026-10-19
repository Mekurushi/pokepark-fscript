#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodegenError {
    FrameTooLarge,
    InvalidFieldProjection {
        ty: fsc_mir::Ty,
        field: fsc_mir::FieldIdx,
    },
    InvalidIndexProjection {
        ty: fsc_mir::Ty,
    },
    IndexOutOfBounds {
        index: usize,
        length: usize,
    },
    InvalidFunctionMapping(fsc_mir::FunctionId),
    MissingFunctionMapping(fsc_mir::FunctionId),
    MissingLocalAllocation(fsc_mir::LocalId),
    MissingStaticMapping(fsc_mir::StaticId),
    InvalidStaticInitializer {
        static_id: fsc_mir::StaticId,
        ty: fsc_mir::Ty,
    },
    UnsupportedStaticType {
        static_id: fsc_mir::StaticId,
        ty: fsc_mir::Ty,
    },
    UnsupportedLocalType {
        local: fsc_mir::LocalId,
        ty: fsc_mir::Ty,
    },
    UnsupportedType(fsc_mir::Ty),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FrameTooLarge => f.write_str("function frame exceeds the VM slot limit"),
            Self::InvalidFieldProjection { ty, field } => {
                write!(f, "field `{field:?}` is not valid for type `{ty:?}`")
            }
            Self::InvalidIndexProjection { ty } => {
                write!(f, "index projection is not valid for type `{ty:?}`")
            }
            Self::IndexOutOfBounds { index, length } => {
                write!(f, "index `{index}` is out of bounds for length `{length}`")
            }
            Self::InvalidFunctionMapping(function) => {
                write!(
                    f,
                    "MIR function `{function:?}` is not mapped to a defined FSB function"
                )
            }
            Self::MissingFunctionMapping(function) => {
                write!(
                    f,
                    "no function mapping exists for MIR function `{function:?}`"
                )
            }
            Self::MissingLocalAllocation(local) => {
                write!(f, "no frame allocation exists for MIR local `{local:?}`")
            }
            Self::MissingStaticMapping(static_id) => {
                write!(f, "no data mapping exists for MIR static `{static_id:?}`")
            }
            Self::InvalidStaticInitializer { static_id, ty } => {
                write!(
                    f,
                    "initializer for MIR static `{static_id:?}` does not match type `{ty:?}`"
                )
            }
            Self::UnsupportedStaticType { static_id, ty } => {
                write!(
                    f,
                    "MIR static `{static_id:?}` of type `{ty:?}` has no FSB data representation"
                )
            }
            Self::UnsupportedLocalType { local, ty } => {
                write!(
                    f,
                    "MIR local `{local:?}` of type `{ty:?}` has no frame representation"
                )
            }
            Self::UnsupportedType(ty) => {
                write!(f, "type `{ty:?}` has no storage representation")
            }
        }
    }
}

pub type CodegenResult<T> = Result<T, CodegenError>;
