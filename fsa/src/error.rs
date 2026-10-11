use crate::lexer::error::LexerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Lexer(LexerError),
    UnexpectedToken { got: String, expected: &'static str },
    UnexpectedEof { expected: &'static str },
    InvalidInteger { value: i64, expected: &'static str },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lexer(error) => error.fmt(f),
            Self::UnexpectedToken { got, expected } => {
                write!(f, "expected {expected}, got `{got}`")
            }
            Self::UnexpectedEof { expected } => {
                write!(f, "unexpected end of file, expected {expected}")
            }
            Self::InvalidInteger { value, expected } => {
                write!(f, "integer `{value}` does not fit in {expected}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(error: LexerError) -> Self {
        Self::Lexer(error)
    }
}

pub type ParseResult<T> = Result<T, ParseError>;
