pub mod ast;
mod error;
pub mod lexer;
pub mod lower;
mod parser;

pub use error::{ParseError, ParseResult};
pub use lower::error::{LowerError, LowerResult};
pub use lower::lower;

pub fn parse(source: &str) -> ParseResult<ast::Script> {
    parser::Parser::new(lexer::tokenize(source)?).parse_script()
}
