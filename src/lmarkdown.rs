use std::io::Read;

use self::{
    char_reader::CharReader,
    parse_error::{ParseError, ParseErrorKind},
};

pub mod char_reader;
pub mod parse_error;

pub struct Lexer<R> {
    reader: CharReader<R>,
}

// https://github.com/markedjs/marked/blob/master/src/Lexer.js
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
impl<R: Read> Lexer<R> {
    pub fn new(reader: CharReader<R>) -> Lexer<R> {
        Lexer { reader }
    }

    fn read_inline_tokens(&mut self, text: &String) -> Result<Vec<Token>, ParseError> {
        let mut tokens = vec![];
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0;
        while pos < chars.len() {
            let c = chars[pos];

            if c == '[' {
                let (text_start, mut text_end, mut href_start, mut href_end) = (pos + 1, 0, 0, 0);
                for (i, c) in chars.iter().enumerate() {
                    let pos = pos + i;
                    match *c {
                        '\n' => break,
                        ']' => text_end = pos,
                        '(' => href_start = pos + 1,
                        ')' => href_end = pos,
                        _ => {}
                    }
                }
                if text_start < text_end && text_end < href_start && href_start < href_end {
                    tokens.push(Token::Link {
                        text: chars[text_start..text_end].iter().collect(),
                        href: chars[href_start..href_end].iter().collect(),
                    });
                    pos = href_end;
                    continue;
                }
            }

            pos += 1;
        }

        return Ok(tokens);
    }

    /// Will first parse a block token (token for a whole line) and then parse for any inline tokens when needed.
    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        match self.reader.peek_char() {
            Err(e) => match e.kind {
                ParseErrorKind::EndOfFile => return Ok(Token::EOF),
                _ => return Err(e),
            },
            Ok(c) => {
                // if c == '#' {
                //     self.reader.peek_string(7)?;
                // }

                let text = self.reader.read_until(|c| c == '\n')?;
                let tokens = self.read_inline_tokens(&text)?;
                return Ok(Token::Text { text, tokens });
            }
        };
    }
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug)]
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
        text: String,
        href: String,
    },
    Text {
        text: String,
        tokens: Vec<Token>,
    },
    EOF,
}

#[derive(Debug)]
pub struct LMarkdown {
    tokens: Vec<Token>,
}

impl LMarkdown {
    pub fn parse(input: impl Read) -> Result<LMarkdown, ParseError> {
        let mut lexer = Lexer::new(CharReader::new(input));

        let mut tokens = vec![];

        loop {
            match lexer.read_token()? {
                Token::EOF => break,
                t => tokens.push(t),
            }
        }

        Ok(LMarkdown { tokens })
    }
}
