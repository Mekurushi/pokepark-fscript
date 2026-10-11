#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexerError {
    pub text: String,
}

impl std::fmt::Display for LexerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid token `{}`", self.text)
    }
}

impl std::error::Error for LexerError {}
