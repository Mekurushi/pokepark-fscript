use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexerError {
    UnknownChar { ch: char, span: Span },
    InternalError { span: Span },
}

impl From<LexerError> for Diagnostic {
    fn from(e: LexerError) -> Self {
        match e {
            LexerError::UnknownChar { ch, span } => {
                Diagnostic::error(Stage::Parse, format!("unknown character `{ch}`"))
                    .with_label(Label::primary(span, "not a valid token"))
            }
            LexerError::InternalError { span } => {
                Diagnostic::error(Stage::Parse, "internal lexer error")
                    .with_label(Label::primary(span, "occurred here"))
            }
        }
    }
}

pub type LexerResult<T> = Result<T, LexerError>;
