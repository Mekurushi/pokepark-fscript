#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LowerError {
    DuplicateFunction(String),
    DuplicateData(String),
    DuplicateLabel { function: String, label: String },
    UnknownFunction(String),
    UnknownData(String),
    UnknownLabel { function: String, label: String },
    MissingTerminator { function: String },
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateFunction(name) => {
                write!(f, "function `{name}` is declared more than once")
            }
            Self::DuplicateData(name) => write!(f, "data `{name}` is declared more than once"),
            Self::DuplicateLabel { function, label } => write!(
                f,
                "label `{label}` is defined more than once in function `{function}`"
            ),
            Self::UnknownFunction(name) => write!(f, "unknown function `{name}`"),
            Self::UnknownData(name) => write!(f, "unknown data `{name}`"),
            Self::UnknownLabel { function, label } => {
                write!(f, "unknown label `{label}` in function `{function}`")
            }
            Self::MissingTerminator { function } => {
                write!(f, "function `{function}` ends without a terminator")
            }
        }
    }
}

impl std::error::Error for LowerError {}

pub type LowerResult<T> = Result<T, LowerError>;
