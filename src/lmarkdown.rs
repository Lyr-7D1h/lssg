use std::io::Read;

use self::{char_reader::CharReader, parse_error::ParseError};

pub mod char_reader;
pub mod parse_error;

pub struct Lexer<R> {
    reader: CharReader<R>,
}

impl<R: Read> Lexer<R> {
    pub fn new(reader: CharReader<R>) -> Lexer<R> {
        Lexer { reader }
    }

    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        let result = match self.reader.peek_char()? {
            // '#' => {}
            // '`' => {
            //     self.reader.peek_string(2),
            // },
            _ => Token::Text {
                text: self.reader.read_until(|c| c == '\n')?,
            },
        };
        return Ok(result);
    }
}

pub enum Token {
    Heading { depth: u8 },
    Bold { text: String },
    Italic { text: String },
    Code { language: String, code: String },
    Link { url: String },
    Text { text: String },
    EOF,
}

pub struct LMarkdown {}

impl LMarkdown {
    pub fn parse(input: impl Read) -> Result<LMarkdown, ParseError> {
        let mut lexer = Lexer::new(CharReader::new(input));
        loop {
            match lexer.read_token()? {
                Token::EOF => break,
                Token::Heading { depth } => todo!(),
                Token::Bold { text } => todo!(),
                Token::Code { language, code } => todo!(),
                Token::Link { url } => todo!(),
                Token::Text { text } => todo!(),
                Token::Italic { text } => todo!(),
            }
        }
        todo!()
    }
}
