use std::{
    collections::{BTreeMap, HashMap},
    io::Read,
};

use log::warn;

use crate::{
    char_reader::CharReader,
    html::{self, parse_html_block},
    parse_error::ParseError,
};

use super::parse_lmarkdown;

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
        // html
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

        // links: https://spec.commonmark.org/0.30/#links
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
                        tokens.push(Token::Link { tokens: text, href });
                        continue;
                    }
                }
            }
        }

        // emphasis: https://spec.commonmark.org/0.30/#emphasis-and-strong-emphasis
        if c == '*' {
            if let Some('*') = reader.peek_char(1)? {
                if let Some(text) = reader.peek_until_match_inclusive_from(2, "**")? {
                    reader.consume(2)?;
                    let text = reader.consume_string(text.len() - 4)?;
                    reader.consume(2)?;
                    tokens.push(Token::Bold { text });
                    continue;
                }
            }
            if let Some(text) = reader.peek_until_from(1, |c| c == '*')? {
                reader.consume(1)?;
                let text = reader.consume_string(text.len() - 1)?;
                reader.consume(1)?;
                tokens.push(Token::Italic { text });
                continue;
            }
        }

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
        Some(c) => {
            // if you start a new block with a newline skip it
            if c == '\n' {
                reader.consume_until_inclusive(|c| c == '\n' || c == '\r')?;
                return Ok(Token::Space);
            }

            // if starts with comment in toml format it is an attribute
            if reader.has_read() == false {
                if c == '<' {
                    if reader.peek_string(4)? == "<!--" {
                        if let Some(comment) = reader.peek_until_match_inclusive("-->")? {
                            match toml::from_str(&comment[4..comment.len() - 3]) {
                                Ok(toml::Value::Table(table)) => {
                                    reader.consume_until_inclusive(|c| c == '>')?;
                                    return Ok(Token::Attributes { table });
                                }
                                Ok(_) => warn!("Attributes is not a table"),
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

            if let Some(heading) = heading(reader)? {
                return Ok(heading);
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
                    let mut reader = CharReader::<&[u8]>::from_string(&content);
                    let tokens = read_inline_html_tokens(&mut reader)?;
                    return Ok(Token::Html {
                        tag,
                        attributes,
                        tokens,
                    });
                }
            }

            if let Some(blockquote) = blockquote(reader)? {
                return Ok(blockquote);
            }

            // https://spec.commonmark.org/0.30/#paragraphs
            let text = sanitize_text(reader.consume_until_match_inclusive("\n")?);
            let tokens = read_inline_tokens(&text)?;
            return Ok(Token::Paragraph { tokens });
        }
    };
}

/// Allow for certain block tokens inside html
pub fn read_inline_html_tokens(
    reader: &mut CharReader<impl Read>,
) -> Result<Vec<Token>, ParseError> {
    let mut tokens = vec![];

    while let Some(_) = reader.peek_char(0)? {
        if let Some(heading) = heading(reader)? {
            tokens.push(heading)
        }
        let text = sanitize_text(reader.consume_until_match_inclusive("\n")?);
        tokens.append(&mut read_inline_tokens(&text)?);
    }

    Ok(tokens)
}

/// Heading (#*{depth} {text})
pub fn heading(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    if reader.peek_char(0)? == Some('#') {
        let chars: Vec<char> = reader.peek_string(7)?.chars().collect();
        let mut depth: u8 = 0;
        for c in chars {
            match c {
                ' ' => break,
                '#' => depth += 1,
                _ => return Ok(None),
            }
        }
        let text: String = sanitize_text(
            reader
                .consume_until_inclusive(|c| c == '\n')?
                .chars()
                .skip(depth as usize + 1)
                .collect(),
        );
        let tokens = read_inline_tokens(&text)?;

        Ok(Some(Token::Heading { depth, tokens }))
    } else {
        Ok(None)
    }
}

// https://spec.commonmark.org/0.30/#block-quotes
pub fn blockquote(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    let mut content = String::new();
    'outer: loop {
        for i in 0..3 {
            match reader.peek_char(i)? {
                Some('>') => {
                    let line = reader.consume_until_inclusive(|c| c == '\n')?;
                    content.push_str(&line[i + 1..line.len() - 1].trim_start());
                }
                Some(' ') => {}
                Some(_) | None => break 'outer,
            }
        }
    }

    if content.len() == 0 {
        return Ok(None);
    }

    let tokens = parse_lmarkdown(content.as_bytes())?;
    return Ok(Some(Token::BlockQuote { tokens }));
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Attributes {
        table: toml::map::Map<String, toml::Value>,
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
    BlockQuote {
        tokens: Vec<Token>,
    },
    Code {
        language: String,
        code: String,
    },
    // Space {
    //     raw: String,
    // },
    Link {
        /// The text portion of a link that contains Tokens
        tokens: Vec<Token>,
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
    /// Indicating of a space between paragraphs
    Space,
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
