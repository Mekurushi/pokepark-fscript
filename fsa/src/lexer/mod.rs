pub mod error;
pub mod token;

use logos::Logos;

use self::error::LexerError;
use self::token::TokenKind;

pub fn tokenize(source: &str) -> Result<Vec<TokenKind>, LexerError> {
    let mut tokens = Vec::new();
    let mut lexer = TokenKind::lexer(source);

    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => tokens.push(token),
            Err(()) => {
                return Err(LexerError {
                    text: lexer.slice().to_owned(),
                });
            }
        }
    }

    Ok(tokens)
}
