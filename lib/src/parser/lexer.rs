use std::{collections::HashMap, io::Read};

use super::{char_reader::CharReader, parse_error::ParseError};

pub struct LMarkdownLexer<R> {
    reader: CharReader<R>,
}

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

// https://github.com/markedjs/marked/blob/master/src/Lexer.js
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// https://marked.js.org/demo/
impl<R: Read> LMarkdownLexer<R> {
    pub fn new(reader: CharReader<R>) -> LMarkdownLexer<R> {
        LMarkdownLexer { reader }
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
                        ')' => {
                            href_end = i;
                            break;
                        }
                        _ => {}
                    }
                }
                if text_start < text_end && text_end < href_start && href_start < href_end {
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

    /// Will first parse a block token (token for a whole line) and then parse for any inline tokens when needed.
    pub fn read_token(&mut self) -> Result<Token, ParseError> {
        match self.reader.peek_char()? {
            None => return Ok(Token::EOF),
            Some(c) => {
                // Heading (#*{depth} {text})
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
                    if ignore == false {
                        let text: String = sanitize_text(
                            self.reader
                                .read_until_inclusive(|c| c == '\n')?
                                .chars()
                                .skip(depth as usize + 1)
                                .collect(),
                        );
                        let tokens = self.read_inline_tokens(&text)?;
                        return Ok(Token::Heading {
                            depth,
                            text,
                            tokens,
                        });
                    }
                }

                if c == '<' {
                    let start_tag = self.reader.read_until_inclusive(|c| c == '>')?;

                    if let Some("!--") = start_tag.get(1..4) {
                        if let Some("-->") = start_tag.get(start_tag.len() - 3..start_tag.len()) {
                            let mut map = HashMap::new();
                            for l in start_tag[4..start_tag.len() - 3].lines() {
                                let mut parts = l.splitn(2, " ");
                                if let Some(key) = parts.next() {
                                    map.insert(key.into(), parts.collect());
                                }
                            }
                            return Ok(Token::Comment {
                                text: start_tag,
                                map,
                            });
                        }
                    }

                    let mut tag = String::new();
                    for c in start_tag[1..start_tag.len() - 1].chars() {
                        match c {
                            ' ' => break,
                            '\n' => break,
                            _ => tag.push(c),
                        }
                    }

                    let mut attributes = HashMap::new();
                    for a in start_tag[1 + tag.len()..start_tag.len() - 1].split(" ") {
                        let mut parts = a.splitn(2, "=");
                        if let Some(k) = parts.next() {
                            if let Some(v) = parts.next() {
                                attributes.insert(k.into(), v.replace("\"", ""));
                            } else {
                                attributes.insert(k.into(), "".into());
                            }
                        }
                    }

                    let mut content = String::new();
                    while let Some(c) = self.reader.read_char()? {
                        if c == '<' {
                            let end_tag_kind = self.reader.peek_string(tag.len() + 2)?;
                            if end_tag_kind == format!("/{tag}>") {
                                self.reader.read_string(tag.len() + 2)?;
                                break;
                            }
                        }

                        content.push(c)
                    }

                    CharReader::new(content.as_bytes());

                    let tokens = self.read_inline_tokens(&content)?;
                    return Ok(Token::Html {
                        tag,
                        attributes,
                        tokens,
                    });
                }

                if c == '\n' {
                    let raw = self.reader.read_until_exclusive(|c| c != '\n')?;
                    return Ok(Token::Space { raw });
                }

                let text = sanitize_text(self.reader.read_until_inclusive(|c| c == '\n')?);
                let tokens = self.read_inline_tokens(&text)?;
                return Ok(Token::Paragraph { tokens });
            }
        };
    }
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone)]
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
    Link {
        text: String,
        href: String,
    },
    Text {
        text: String,
    },
    Html {
        tag: String,
        attributes: HashMap<String, String>,
        tokens: Vec<Token>,
    },
    Comment {
        text: String,
        /// Starting a comment line with a certain key will be used as keyword later in the renderer
        map: HashMap<String, String>,
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
