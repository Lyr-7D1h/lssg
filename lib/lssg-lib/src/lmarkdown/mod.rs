use std::io::Read;

use self::{char_reader::CharReader, parse_error::ParseError};

pub mod char_reader;
mod lexer;
pub use lexer::*;
pub mod parse_error;

pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut lexer = LMarkdownLexer::new(CharReader::new(input));

    let mut tokens = vec![];

    loop {
        match lexer.read_token()? {
            Token::EOF => break,
            t => tokens.push(t),
        }
    }

    Ok(tokens)
}
