use std::{collections::HashMap, io::Read};

use log::warn;

use crate::{
    char_reader::CharReader,
    html::{self, parse_html_block},
    parse_error::ParseError,
};

/// Remove any tailing new line or starting and ending spaces
fn sanitize_text(text: String) -> String {
    let mut lines = vec![];
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.len() > 0 {
            lines.push(trimmed);
        }
    }

    return lines.join("\n");
}

fn read_inline_tokens(text: &String) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::<&[u8]>::from_string(text);

    let mut tokens = vec![];
    while let Some(c) = reader.peek_char(0)? {
        if c == '<' {
            // inline comment
            if let Some(text) = reader.peek_until_match_inclusive("-->")? {
                reader.consume(4)?; // skip start
                let text = reader.consume_string(text.len() - 4 - 3)?;
                reader.consume(3)?; // skip end
                tokens.push(Token::Comment { raw: text });
                continue;
            }

            if let Some((tag, attributes, content)) = html::parse_html_block(&mut reader)? {
                let content = sanitize_text(content);
                tokens.push(Token::Html {
                    tag,
                    attributes,
                    tokens: read_inline_tokens(&content)?,
                });
                continue;
            }
        }

        // https://spec.commonmark.org/0.30/#links
        if c == '[' {
            if let Some(raw_text) = reader.peek_until_from(1, |c| c == ']')? {
                let href_start = 1 + raw_text.len();
                if let Some('(') = reader.peek_char(href_start)? {
                    if let Some(raw_href) = reader.peek_until_from(href_start + 1, |c| c == ')')? {
                        reader.consume(1)?;
                        let text = reader.consume_string(raw_text.len() - 1)?;
                        reader.consume(2)?;
                        let href = reader.consume_string(raw_href.len() - 1)?;
                        reader.consume(1)?;
                        let text = read_inline_tokens(&text)?;
                        tokens.push(Token::Link { text, href });
                        continue;
                    }
                }
            }
        }

        if c == '*' {}

        let c = reader.consume_char().unwrap().expect("has to be a char");
        if let Some(Token::Text { text }) = tokens.last_mut() {
            text.push(c)
        } else {
            tokens.push(Token::Text { text: c.into() })
        }
    }

    return Ok(tokens);
}

// official spec: https://spec.commonmark.org/0.30/
// https://github.com/markedjs/marked/blob/master/src/Lexer.ts
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// demo: https://marked.js.org/demo/
// demo: https://spec.commonmark.org/dingus/
/// A function to get the next markdown token using recrusive decent.
/// Will first parse a block token (token for a whole line and then parse for any inline tokens when needed.
pub fn read_token(reader: &mut CharReader<impl Read>) -> Result<Token, ParseError> {
    match reader.peek_char(0)? {
        None => return Ok(Token::EOF),
        Some(mut c) => {
            // if you start a new block with a newline skip it
            if c == '\n' {
                reader.consume_until_inclusive(|c| c == '\n' || c == '\r')?;
                c = match reader.peek_char(0)? {
                    Some(c) => c,
                    None => return Ok(Token::EOF),
                }
            }

            // if starts with comment in toml format it is an attribute
            if reader.has_read() == false {
                if c == '<' {
                    if reader.peek_string(4)? == "<!--" {
                        if let Some(comment) = reader.peek_until_match_inclusive("-->")? {
                            match toml::from_str(&comment[4..comment.len() - 3]) {
                                Ok(toml) => {
                                    reader.consume_until_inclusive(|c| c == '>')?;
                                    return Ok(Token::Attributes { toml });
                                }
                                Err(e) => warn!("Not parsing possible Attributes: {e}"),
                            }
                        }
                    }
                }
                if let Some((tag, attributes, content)) = parse_html_block(reader)? {
                    let tokens = read_inline_tokens(&content)?;
                    return Ok(Token::Html {
                        tag,
                        attributes,
                        tokens,
                    });
                }
            }

            // Heading (#*{depth} {text})
            if c == '#' {
                let chars: Vec<char> = reader.peek_string(7)?.chars().collect();
                let mut ignore = false;
                let mut depth: u8 = 0;
                for c in chars {
                    match c {
                        ' ' => break,
                        '#' => depth += 1,
                        _ => ignore = true,
                    }
                }
                if ignore == false {
                    let text: String = sanitize_text(
                        reader
                            .consume_until_inclusive(|c| c == '\n')?
                            .chars()
                            .skip(depth as usize + 1)
                            .collect(),
                    );
                    let tokens = read_inline_tokens(&text)?;
                    return Ok(Token::Heading { depth, tokens });
                }
            }

            if c == '<' {
                // comment
                if "<!--" == reader.peek_string(4)? {
                    if let Some(text) = reader.peek_until_match_inclusive("-->")? {
                        reader.consume(4)?; // skip start
                        let text = reader.consume_string(text.len() - 4 - 3)?;
                        reader.consume(3)?; // skip end
                        return Ok(Token::Comment { raw: text });
                    }
                }

                if let Some((tag, attributes, content)) = html::parse_html_block(reader)? {
                    let content = sanitize_text(content);
                    let tokens = read_inline_tokens(&content)?;
                    return Ok(Token::Html {
                        tag,
                        attributes,
                        tokens,
                    });
                }
            }

            // https://spec.commonmark.org/0.30/#paragraphs
            let text = sanitize_text(reader.consume_until_match_inclusive("\n")?);
            let tokens = read_inline_tokens(&text)?;
            return Ok(Token::Paragraph { tokens });
        }
    };
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Attributes {
        toml: toml::Value,
    },
    Heading {
        /// 0-6
        depth: u8,
        tokens: Vec<Token>,
    },
    Html {
        tag: String,
        attributes: HashMap<String, String>,
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
    // Space {
    //     raw: String,
    // },
    Link {
        text: Vec<Token>,
        href: String,
    },
    Text {
        text: String,
    },
    Comment {
        raw: String,
    },
    Break {
        raw: String,
    },
    EOF,
}

impl Token {
    pub fn is_text(&self) -> bool {
        match self {
            Token::Heading { .. }
            | Token::Paragraph { .. }
            | Token::Bold { .. }
            | Token::Italic { .. }
            | Token::Code { .. }
            | Token::Link { .. }
            | Token::Text { .. }
            | Token::Html { .. } => true,
            _ => false,
        }
    }
}
