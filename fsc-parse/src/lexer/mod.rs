pub mod error;
pub mod token;

use crate::lexer::error::LexerError;
use crate::lexer::token::{LexOutput, Token, TokenKind};
use fsc_diagnostics::Span;
use logos::Logos;

pub fn tokenize(src: &str) -> LexOutput {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut lex = TokenKind::lexer(src);

    while let Some(result) = lex.next() {
        let span = lex.span();
        match result {
            Ok(kind) => tokens.push(Token {
                kind,
                span: Span::new(span.start, span.end),
            }),
            Err(()) => match lex.slice().chars().next() {
                Some(ch) => errors.push(LexerError::UnknownChar {
                    ch,
                    span: Span::new(span.start, span.end),
                }),
                None => errors.push(LexerError::InternalError {
                    span: Span::new(span.start, span.end),
                }),
            },
        }
    }

    LexOutput { tokens, errors }
}
