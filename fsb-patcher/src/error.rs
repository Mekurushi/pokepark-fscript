use fsb_assembler::error::{AssemblerError, BinaryReadError};
use fsc_diagnostics::Diagnostic;

#[derive(Debug, PartialEq, Eq)]
pub enum PatchFailure {
    InvalidOriginalBinary(BinaryReadError),
    InvalidPatchSource(Vec<Diagnostic>),
    Assembly(AssemblerError),
    InvalidFunctionAddress {
        function_name: String,
        address: u32,
    },
    ConflictingExternalSymbol {
        function_name: String,
        embedded_offset: u32,
        external_offset: u32,
    },
    MissingExternalSymbol {
        function_name: String,
    },
}

impl std::fmt::Display for PatchFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidOriginalBinary(error) => {
                write!(f, "could not read original FSB: {error}")
            }
            Self::InvalidPatchSource(_) => f.write_str("patch source contains errors"),
            Self::Assembly(error) => write!(f, "could not assemble patch: {error}"),
            Self::InvalidFunctionAddress {
                function_name,
                address,
            } => write!(
                f,
                "function '{function_name}' has invalid address {address:#x}"
            ),
            Self::ConflictingExternalSymbol {
                function_name,
                embedded_offset,
                external_offset,
            } => write!(
                f,
                "external function '{function_name}' resolves to code offset {external_offset:#x}, but the embedded symbol resolves to {embedded_offset:#x}"
            ),
            Self::MissingExternalSymbol { function_name } => write!(
                f,
                "external function '{function_name}' is missing from the symbol table"
            ),
        }
    }
}

impl std::error::Error for PatchFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidOriginalBinary(error) => Some(error),
            Self::Assembly(error) => Some(error),
            Self::InvalidPatchSource(_)
            | Self::InvalidFunctionAddress { .. }
            | Self::ConflictingExternalSymbol { .. }
            | Self::MissingExternalSymbol { .. } => None,
        }
    }
}

impl From<BinaryReadError> for PatchFailure {
    fn from(error: BinaryReadError) -> Self {
        Self::InvalidOriginalBinary(error)
    }
}

impl From<AssemblerError> for PatchFailure {
    fn from(error: AssemblerError) -> Self {
        Self::Assembly(error)
    }
}

#[derive(Debug)]
pub struct SymbolTableParseError {
    pub(crate) source: toml::de::Error,
}

#[derive(Debug)]
pub struct SymbolTableSerializeError {
    pub(crate) source: toml::ser::Error,
}

impl std::fmt::Display for SymbolTableSerializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(f)
    }
}

impl std::error::Error for SymbolTableSerializeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

impl std::fmt::Display for SymbolTableParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(f)
    }
}

impl std::error::Error for SymbolTableParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
