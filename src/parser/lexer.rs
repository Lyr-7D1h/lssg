use std::io::Read;

use super::{char_reader::CharReader, parse_error::ParseError};

pub struct Lexer<R> {
    reader: CharReader<R>,
}

// https://github.com/markedjs/marked/blob/master/src/Lexer.js
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// https://marked.js.org/demo/
impl<R: Read> Lexer<R> {
    pub fn new(reader: CharReader<R>) -> Lexer<R> {
        Lexer { reader }
    }

    fn read_inline_tokens(&mut self, text: &String) -> Result<Vec<Token>, ParseError> {
        let mut tokens = vec![];
        let chars: Vec<char> = text.chars().collect();
        let mut pos = 0;
        let mut text = String::new();
        while pos < chars.len() {
            let c = chars[pos];

            if c == '[' {
                let (text_start, mut text_end, mut href_start, mut href_end) = (pos + 1, 0, 0, 0);
                for i in pos..chars.len() {
                    match chars[i] {
                        '\n' => break,
                        ']' => text_end = i,
                        '(' => href_start = i + 1,
                        ')' => href_end = i,
                        _ => {}
                    }
                }
                if text_start < text_end && text_end < href_start && href_start < href_end {
                    if text.len() > 0 {
                        tokens.push(Token::Text { text: text.clone() });
                        text.clear();
                    }
                    tokens.push(Token::LinkRef {
                        text: chars[text_start..text_end].iter().collect(),
                        href: chars[href_start..href_end].iter().collect(),
                    });
                    pos = href_end + 1;
                    continue;
                }
            }

            text.push(chars[pos]);
            pos += 1;
        }
        if text.len() > 0 {
            tokens.push(Token::Text { text: text.clone() });
            text.clear();
        }

        return Ok(tokens);
    }

    /// Will first parse a block token (token for a whole line) and then parse for any inline tokens when needed.
    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        match self.reader.peek_char()? {
            None => return Ok(Token::EOF),
            Some(c) => {
                // Heading
                if c == '#' {
                    let chars: Vec<char> = self.reader.peek_string(7)?.chars().collect();
                    let mut ignore = false;
                    let mut depth: u8 = 0;
                    for c in chars {
                        match c {
                            ' ' => break,
                            '#' => depth += 1,
                            _ => ignore = true,
                        }
                    }
                    let text = self
                        .reader
                        .read_until(|c| c == '\n')?
                        .chars()
                        .skip(depth as usize + 1)
                        .collect();
                    let tokens = self.read_inline_tokens(&text)?;
                    if ignore == false {
                        return Ok(Token::Heading {
                            depth,
                            text,
                            tokens,
                        });
                    }
                }

                if c == '\n' {
                    let raw = self.reader.read_until(|c| c != '\n')?;
                    return Ok(Token::Space { raw });
                }

                let text = self.reader.read_until(|c| c == '\n')?;
                let tokens = self.read_inline_tokens(&text)?;
                return Ok(Token::Paragraph { tokens });
            }
        };
    }
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug)]
pub enum Token {
    Heading {
        /// 0-6
        depth: u8,
        text: String,
        tokens: Vec<Token>,
    },
    /// Anything that is not an already declared inline element
    Paragraph {
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
    Space {
        raw: String,
    },
    LinkRef {
        text: String,
        href: String,
    },
    Link {
        rel: String,
        text: String,
        href: String,
    },
    Text {
        text: String,
    },
    EOF,
}
