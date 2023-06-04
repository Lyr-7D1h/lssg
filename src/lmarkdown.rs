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

    fn read_inline_tokens(&mut self, text: CharReader<R>) -> Result<Vec<Token>, ParseError> {}

    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        let result = match self.reader.peek_char()? {
            // '#' => {}
            // '`' => {
            //     self.reader.peek_string(2),
            // },
            _ => {
                let text = self.reader.read_until(|c| c == '\n')?;
                let tokens = self.read_inline_tokens(CharReader::new(&text[..]))?;
                Token::Text {
                    text: self.reader.read_until(|c| c == '\n')?,
                    tokens,
                }
            }
        };
        return Ok(result);
    }
}

pub enum Token {
    Heading {
        depth: u8,
        text: String,
        tokens: Vec<Token>,
    },
    Bold {
        text: String,
    },
    Italic {
        text: String,
    },
    Code {
        language: String,
        code: String,
    },
    Link {
        url: String,
    },
    Text {
        text: String,
        tokens: Vec<Token>,
    },
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
