pub mod ast;
mod error;
pub mod lexer;
pub mod parser;

pub use error::{ParseError, ParseResult};

pub fn parse(source: &str) -> ParseResult<ast::Script> {
    parser::Parser::new(lexer::tokenize(source)?).parse_script()
}
