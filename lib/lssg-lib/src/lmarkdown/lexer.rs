use std::{collections::HashMap, io::Read};

use log::warn;

use crate::{
    char_reader::CharReader,
    html::{self, Html},
    parse_error::ParseError,
};

/// Remove any tailing new line
fn sanitize_text(text: String) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if let Some(c) = chars.last() {
        if c == &'\n' {
            chars.pop();
        }
    }
    chars.into_iter().collect()
}

fn html_to_token(html: Html) -> Result<Token, ParseError> {
    match html {
        Html::Comment { text } => Ok(Token::Comment { text }),
        Html::Text { text } => {
            let mut reader = CharReader::new(text.as_bytes());
            return read_token(&mut reader);
        }
        Html::Element {
            tag,
            attributes,
            children,
        } => {
            let tokens = children
                .into_iter()
                .map(|c| html_to_token(c))
                .collect::<Result<Vec<Token>, ParseError>>()?;
            return Ok(Token::Html {
                tag,
                attributes,
                tokens,
            });
        }
    }
}

fn read_inline_tokens(text: &String) -> Result<Vec<Token>, ParseError> {
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
                    ')' => {
                        href_end = i;
                        break;
                    }
                    _ => {}
                }
            }
            if text_start <= text_end && text_end <= href_start && href_start < href_end {
                if text.len() > 0 {
                    tokens.push(Token::Text { text: text.clone() });
                    text.clear();
                }
                tokens.push(Token::Link {
                    text: chars[text_start..text_end].iter().collect(),
                    href: chars[href_start..href_end].iter().collect(),
                });
                pos = href_end + 1;
                continue;
            }
        }

        if c == '<' {
            // TODO support inline html and comments
            // let (start_tag, mut start_tag_end) = (pos, 0);
            // for i in pos..chars.len() {
            //     match chars[i] {
            //         '\n' => break,
            //         '>' => start_tag_end = i,
            //         _ => {}
            //     }
            // }
            // let mut tag_kind = String::new();
            // for i in start_tag + 1..start_tag_end {
            //     match chars[i] {
            //         ' ' => break,
            //         c => tag_kind.push(c),
            //     };
            // }

            // let (mut end_tag_start, mut end_tag_end) = (0, 0);
            // if !tag_kind.is_empty() {
            //     for i in start_tag_end..chars.len() {
            //         if chars[i] == '<' {
            //             if let Some(c) = chars.get(i + 1) {
            //                 if c == &'/' {
            //                     let exit_tag = chars[i..i + tag_kind.len()]
            //                         .into_iter()
            //                         .collect::<String>();
            //                     if exit_tag == tag_kind {
            //                         end_tag_start = i;
            //                         end_tag_end = i + tag_kind.len();
            //                         break;
            //                     }
            //                 }
            //             }
            //         }
            //     }
            // }

            // if start_tag < start_tag_end && end_tag_start < end_tag_end {
            //     let tag: String = chars[start_tag + 1..start_tag_end].into_iter().collect();
            //     // pos = end_tag_end;
            //     println!("Kind {tag_kind}");
            // }
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

// https://spec.commonmark.org/dingus/
// https://github.com/markedjs/marked/blob/master/src/Lexer.js
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// https://marked.js.org/demo/
/// A function to get the next markdown token using recrusive decent.
/// Will first parse a block token (token for a whole line and then parse for any inline tokens when needed.
pub fn read_token(reader: &mut CharReader<impl Read>) -> Result<Token, ParseError> {
    match reader.peek_char(0)? {
        None => return Ok(Token::EOF),
        Some(c) => {
            // if starts with comment in toml format it is an attribute
            if reader.has_read() == false {
                if c == '<' {
                    if reader.peek_string(3)? == "!--" {
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
                    return Ok(Token::Heading {
                        depth,
                        text,
                        tokens,
                    });
                }
            }

            if c == '<' {
                // comment
                if "<!--" == reader.peek_string(4)? {
                    if let Some(text) = reader.peek_until_match_inclusive("-->")? {
                        reader.consume(4)?; // skip start
                        let text = reader.consume_string(text.len() - 3)?;
                        reader.consume(3)?; // skip end
                        return Ok(Token::Comment { text });
                    }
                }

                if let Some(start_tag) = reader.peek_until(|c| c == '>')? {
                    let mut tag = String::new();
                    for c in start_tag[1..start_tag.len() - 1].chars() {
                        match c {
                            ' ' => break,
                            '\n' => break,
                            _ => tag.push(c),
                        }
                    }

                    let mut raw_html = start_tag;
                    if let Some(content) =
                        reader.peek_until_match_inclusive(&format!("</{tag}>"))?
                    {
                        raw_html.push_str(&content);
                        let html = html::parse_html(content.as_bytes())?.into_iter().next().expect("Has to contain a single html element");

                        return html_to_token(html);
                    }
                }
            }

            if c == '\n' {
                let raw = reader.consume_until_inclusive(|c| c != '\n' && c != '\r')?;
                return Ok(Token::Space { raw });
            }

            let text = sanitize_text(reader.consume_until_inclusive(|c| c == '\n' || c == '\r')?);
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
        text: String,
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
    Space {
        raw: String,
    },
    Link {
        text: String,
        href: String,
    },
    Text {
        text: String,
    },
    Comment {
        text: String,
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
