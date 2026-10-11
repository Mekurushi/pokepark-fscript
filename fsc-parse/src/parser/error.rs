use crate::lexer::error::LexerError;
use fsc_diagnostics::{Diagnostic, Label, Span, Stage};

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken {
        got: String,
        expected: &'static str,
        span: Span,
    },
    UnexpectedEof {
        expected: &'static str,
        span: Span,
    },
    Lex(Vec<LexerError>),
}

impl ParseError {
    #[must_use]
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        match self {
            Self::Lex(errors) if errors.is_empty() => {
                vec![Diagnostic::error(Stage::Parse, "unknown lexer error")]
            }
            Self::Lex(errors) => errors.into_iter().map(Diagnostic::from).collect(),
            ParseError::UnexpectedToken {
                got,
                expected,
                span,
            } => vec![
                Diagnostic::error(Stage::Parse, format!("expected {expected}, found {got}"))
                    .with_label(Label::primary(span, format!("expected {expected}"))),
            ],

            ParseError::UnexpectedEof { expected, span } => {
                vec![
                    Diagnostic::error(
                        Stage::Parse,
                        format!("expected {expected}, found end of file"),
                    )
                    .with_label(Label::primary(span, "file ends here")),
                ]
            }
        }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lexer_error_collection_has_a_fallback_diagnostic() {
        let diagnostics = ParseError::Lex(Vec::new()).into_diagnostics();

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].stage(), Stage::Parse);
        assert!(diagnostics[0].labels().is_empty());
    }
}
