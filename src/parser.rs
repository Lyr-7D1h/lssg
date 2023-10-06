use std::io::Read;

use self::{
    char_reader::CharReader,
    lexer::{LMarkdownLexer, Token},
    parse_error::ParseError,
};

pub mod char_reader;
pub mod lexer;
pub mod parse_error;

#[derive(Debug)]
pub struct LMarkdownParser {}

impl LMarkdownParser {
    pub fn parse(input: impl Read) -> Result<Vec<Token>, ParseError> {
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
}
