// Assembler Error
#[derive(Debug, PartialEq, Eq)]
pub enum AssemblerError {
    AddressOverflow,
    InvalidB40Char(char),
    InvalidCodeOffset(u32),
    OperandOutOfRange(i32),
    UndefinedSymbol(String),
    DuplicateSymbol(String),
    MissingTerminator { function: String, block: String },
    InvalidStringTable,
    StringTableFull,
    UndefinedString(String),
    SectionTooLarge(&'static str),
}

pub type AssemblerResult<T> = Result<T, AssemblerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryReadError {
    FileTooShort { minimum: usize, actual: usize },
    InvalidB40 { location: String, value: u32 },
    InvalidSectionLayout,
    InvalidSymbolTableSize { size: usize },
    InvalidSymbolOffset { offset: u32 },
}

pub type BinaryReadResult<T> = Result<T, BinaryReadError>;

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssemblerError::AddressOverflow => f.write_str("code address exceeds 32-bit range"),
            AssemblerError::InvalidB40Char(c) => write!(
                f,
                "invalid character '{c}' in script name, allowed: ' 0-9 A-Z _ - /`"
            ),
            AssemblerError::InvalidCodeOffset(offset) => {
                write!(
                    f,
                    "code offset {offset:#x} is unaligned or outside the code section"
                )
            }
            AssemblerError::OperandOutOfRange(n) => {
                write!(f, "operand {n:#x} out of 16-bit signed range")
            }
            AssemblerError::UndefinedSymbol(name) => write!(f, "undefined symbol '{name}'"),
            AssemblerError::DuplicateSymbol(name) => write!(f, "duplicated symbol '{name}'"),
            AssemblerError::MissingTerminator { function, block } => {
                write!(
                    f,
                    "block '{block}' in function '{function}' has no terminator"
                )
            }
            AssemblerError::InvalidStringTable => {
                f.write_str("string table is not null-terminated UTF-8")
            }
            AssemblerError::StringTableFull => f.write_str("string table capacity exceeded"),
            AssemblerError::UndefinedString(value) => {
                write!(f, "undefined string '{value}'")
            }
            AssemblerError::SectionTooLarge(section) => {
                write!(f, "section '{section}' exceeds maximum size")
            }
        }
    }
}

impl std::error::Error for AssemblerError {}

impl std::fmt::Display for BinaryReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileTooShort { minimum, actual } => {
                write!(
                    f,
                    "FSB is too short: expected at least {minimum} bytes, got {actual}"
                )
            }
            Self::InvalidB40 { location, value } => {
                write!(f, "invalid B40 value {value:#x} in {location}")
            }
            Self::InvalidSectionLayout => f.write_str("FSB section pointers are invalid"),
            Self::InvalidSymbolTableSize { size } => {
                write!(f, "FSB symbol table size {size} is not a multiple of 12")
            }
            Self::InvalidSymbolOffset { offset } => {
                write!(
                    f,
                    "FSB symbol offset {offset:#x} is outside the code section"
                )
            }
        }
    }
}

impl std::error::Error for BinaryReadError {}
