use fsb_assembler::error::AssemblerError;
use fsc_sema::hir::LocalId;

#[derive(Debug, PartialEq)]
pub enum CodegenError {
    // TODO: more check errors should wander into check module
    UndeclaredVariable(String),

    AlreadyDeclared(String),

    FrameTooLarge,

    UnknownLocal(LocalId),

    DuplicateLocal(LocalId),

    InvalidLocalType(LocalId),

    Assembler(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndeclaredVariable(name) => {
                write!(f, "undeclared variable `{name}`")
            }
            Self::AlreadyDeclared(name) => {
                write!(f, "variable `{name}` already declared in this scope")
            }
            Self::FrameTooLarge => {
                write!(f, "function frame exceeds the VM slot limit")
            }
            Self::UnknownLocal(local) => {
                write!(f, "no frame allocation exists for local `{local:?}`")
            }
            Self::DuplicateLocal(local) => {
                write!(f, "local `{local:?}` is declared more than once")
            }
            Self::InvalidLocalType(local) => {
                write!(f, "local `{local:?}` has no frame storage representation")
            }
            Self::Assembler(msg) => {
                write!(f, "assembler error: {msg}")
            }
        }
    }
}

pub type CodegenResult<T> = Result<T, CodegenError>;

impl From<AssemblerError> for CodegenError {
    fn from(e: AssemblerError) -> Self {
        Self::Assembler(e.to_string())
    }
}
