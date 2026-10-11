use crate::ast::Script;
use crate::parser::error::{ParseError, ParseResult};
use crate::parser::{Parser, TokenStream};

pub mod ast;
pub mod lexer;
pub mod parser;

pub fn parse(source: &str) -> ParseResult<Script> {
    let lex_output = lexer::tokenize(source);

    if !lex_output.errors.is_empty() {
        return Err(ParseError::Lex(lex_output.errors));
    }

    let ts = TokenStream::new(lex_output.tokens, source.to_string());
    let mut parser = Parser::new(ts);
    let mut items = Vec::new();

    while !parser.is_at_end() {
        items.push(parser.parse_item()?);
    }

    Ok(Script { items })
}
