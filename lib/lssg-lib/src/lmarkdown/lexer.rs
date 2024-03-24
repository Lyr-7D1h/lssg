use std::{collections::HashMap, io::Read};

use log::warn;
use virtual_dom::Html;

use crate::{char_reader::CharReader, parse_error::ParseError};

// official spec: https://spec.commonmark.org/0.30/
// https://github.com/markedjs/marked/blob/master/src/Lexer.ts
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// demo: https://marked.js.org/demo/
// demo: https://spec.commonmark.org/dingus/
/// A function to get the next markdown token using recursive decent.
/// Will first parse a block token (token for one or multiple lines) and then parse for any inline tokens when needed.
pub fn read_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {}

fn read_block_token(
    c: char,
    blank_line: bool,
    reader: &mut CharReader<impl Read>,
    tokens: &mut Vec<Token>,
) -> Result<Option<Token>, ParseError> {
    // if starts with comment in toml format it is an attribute
    if reader.has_read() == false {
        if c == '<' {
            if reader.peek_string(4)? == "<!--" {
                if let Some(comment) = reader.peek_until_match_inclusive("-->")? {
                    match toml::from_str(&comment[4..comment.len() - 3]) {
                        Ok(toml::Value::Table(table)) => {
                            reader.consume_until_inclusive(|c| c == '>')?;
                            return Ok(Some(Token::Attributes { table }));
                        }
                        Ok(_) => warn!("Attributes is not a table"),
                        Err(e) => warn!("Not parsing possible Attributes: {e}"),
                    }
                }
            }
        }
    }

    if let Some(heading) = heading(reader)? {
        return Ok(Some(heading));
    }

    if c == '<' {
        // comment
        if let Some(Html::Comment { text: raw }) = html_comment(reader)? {
            return Ok(Some(Token::Comment { raw }));
        }

        if let Some((tag, attributes, content)) = html_element(reader)? {
            let mut reader = CharReader::<&[u8]>::from_string(&content);
            let tokens = read_inline_html_tokens(&mut reader)?;
            return Ok(Some(Token::Html {
                tag,
                attributes,
                tokens,
            }));
        }
    }

    if let Some(setext) = setext_heading(reader, tokens)? {
        return Ok(Some(setext));
    }

    if let Some(tbreak) = thematic_break(reader)? {
        return Ok(Some(tbreak));
    }

    if let Some(list) = bullet_list(reader)? {
        return Ok(Some(list));
    }

    if let Some(list) = ordered_list(reader)? {
        return Ok(Some(list));
    }

    // list item takes precedence
    if let Some(code) = indented_code(reader, tokens, blank_line)? {
        return Ok(Some(code));
    }

    if let Some(code) = fenced_code(reader)? {
        return Ok(Some(code));
    }

    if let Some(blockquote) = blockquote(reader)? {
        return Ok(Some(blockquote));
    }

    // TODO https://spec.commonmark.org/0.30/#link-reference-definitions

    // https://spec.commonmark.org/0.30/#paragraphs
    // let mut text = reader.consume_until_match_inclusive("\n\n")?;
    // let hard_break = if text.ends_with("  ") {
    //     true
    // } else if text.ends_with("\\") {
    //     text.pop();
    //     true
    // } else {
    //     false
    // };
    let inline_tokens = read_inline_tokens(reader)?;
    // add to prev p if there isn't a blank line in between
    // if let Some(Token::Paragraph {
    //     tokens: last_tokens,
    //     hard_break: last_hard_break,
    // }) = tokens.last_mut()
    // {
    //     if !blank_line {
    //         if *last_hard_break {
    //             // https://spec.commonmark.org/0.30/#hard-line-breaks
    //             last_tokens.push(Token::HardBreak);
    //         } else {
    //             // https://spec.commonmark.org/0.30/#soft-line-breaks
    //             last_tokens.push(Token::SoftBreak);
    //         }
    //         last_tokens.append(&mut inline_tokens);
    //         *last_hard_break = hard_break;
    //
    //         return Ok(None);
    //     }
    // }
    return Ok(Some(Token::Paragraph {
        tokens: inline_tokens,
    }));
}

fn read_inline_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {
    let mut tokens = vec![];
    'outer: while let Some(c) = reader.peek_char(0)? {
        // html
        if c == '<' {
            // comment
            if let Some(Html::Comment { text: raw }) = html_comment(reader)? {
                tokens.push(Token::Comment { raw });
                continue;
            }

            if let Some((tag, attributes, content)) = html_element(reader)? {
                let content = sanitize_text(content);
                tokens.push(Token::Html {
                    tag,
                    attributes,
                    tokens: read_inline_tokens(&mut CharReader::new(content.as_bytes()))?,
                });
                continue;
            }

            // https://spec.commonmark.org/0.30/#autolinks
            if let Some(link) = reader.peek_until_exclusive_from(1, |c| c == '>')? {
                let mut valid = false;
                for c in link.chars() {
                    match c {
                        '<' | '>' | ' ' => {
                            valid = false;
                            break;
                        }
                        // must contain scheme
                        ':' => {
                            valid = true;
                        }
                        _ => {}
                    }
                }
                if valid {
                    reader.consume(1)?;
                    let text = reader.consume_string(link.len())?;
                    reader.consume(1)?;
                    tokens.push(Token::Link {
                        tokens: vec![Token::Text { text }],
                        href: link,
                        title: None,
                    });
                    continue;
                }
            }
        }

        // https://spec.commonmark.org/0.30/#code-spans
        if c == '`' {
            let mut backtick_count = 1;
            while let Some('`') = reader.peek_char(backtick_count)? {
                backtick_count += 1;
            }

            let mut i = backtick_count;
            let mut count = 0;
            while let Some(c) = reader.peek_char(i)? {
                match c {
                    '`' => {
                        count += 1;

                        // skip if next is backtick
                        if let Ok(Some('`')) = reader.peek_char(i + 1) {
                            i += 1;
                            continue;
                        }

                        if count == backtick_count {
                            reader.consume(backtick_count)?;
                            let mut text = reader.consume_string(i + 1 - backtick_count * 2)?;
                            // remove leading and ending space if not only contained with spaces
                            if text.starts_with(" ") && text.ends_with(" ") {
                                if let Some(_) = text.find(char::is_alphabetic) {
                                    text = text[1..text.len() - 1].to_string();
                                }
                            }
                            reader.consume(backtick_count)?;
                            tokens.push(Token::Code { info: None, text });
                            continue 'outer;
                        }
                    }
                    _ => count = 0,
                }
                i += 1;
            }
        }

        // https://spec.commonmark.org/0.30/#images
        if c == '!' {
            if let Some('[') = reader.peek_char(1)? {
                if let Some(raw_text) = reader.peek_until_inclusive_from(2, |c| c == ']')? {
                    let href_start = 2 + raw_text.len();
                    if let Some('(') = reader.peek_char(href_start)? {
                        if let Some(raw_href) =
                            reader.peek_until_inclusive_from(href_start + 1, |c| c == ')')?
                        {
                            reader.consume(2)?;
                            let text = reader.consume_string(raw_text.len() - 1)?;
                            reader.consume(2)?;
                            let src = reader.consume_string(raw_href.len() - 1)?;
                            let src = sanitize_text(src);

                            // https://spec.commonmark.org/0.30/#link-title
                            let title = if let Some(start_title) = src.find(" ") {
                                let title = &src[start_title..src.len()];

                                if ((title.starts_with("\"") && title.ends_with("\""))
                                    || (title.starts_with("\'") && title.ends_with("\'")))
                                    && title.len() >= 2
                                {
                                    Some(title.to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            reader.consume(1)?;
                            let alt = read_inline_tokens(&mut CharReader::new(text.as_bytes()))?;
                            tokens.push(Token::Image {
                                tokens: alt,
                                src,
                                title,
                            });
                            continue;
                        }
                    }
                }
            }
        }

        // links: https://spec.commonmark.org/0.30/#links
        if c == '[' {
            let mut indent = 1;
            let mut i = 1;
            while let Ok(Some(c)) = reader.peek_char(i) {
                i += 1;
                match c {
                    '[' => {
                        if let Ok(Some('!')) = reader.peek_char(i - 1) {
                            continue;
                        }
                        indent += 1;
                    }
                    ']' => {
                        indent -= 1;
                        if indent == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if indent == 0 {
                if let Some('(') = reader.peek_char(i)? {
                    if let Some(raw_href) = reader.peek_until_inclusive_from(i + 1, |c| c == ')')? {
                        reader.consume(1)?;
                        let text = reader.consume_string(i - 2)?;
                        reader.consume(2)?;
                        let href = reader.consume_string(raw_href.len() - 1)?;
                        reader.consume(1)?;
                        let text = sanitize_text(text);
                        let text = read_inline_tokens(&mut CharReader::new(text.as_bytes()))?;

                        // https://spec.commonmark.org/0.30/#link-title
                        let title = if let Some(start_title) = href.find(" ") {
                            let title = &href[start_title..href.len()];

                            if ((title.starts_with("\"") && title.ends_with("\""))
                                || (title.starts_with("\'") && title.ends_with("\'")))
                                && title.len() >= 2
                            {
                                Some(title.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        tokens.push(Token::Link {
                            tokens: text,
                            href,
                            title,
                        });
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
                    let text = reader.consume_string(text.len() - 2)?;
                    reader.consume(2)?;
                    tokens.push(Token::Bold { text });
                    continue;
                }
            }
            if let Some(text) = reader.peek_until_inclusive_from(1, |c| c == '*')? {
                reader.consume(1)?;
                let text = reader.consume_string(text.len() - 1)?;
                reader.consume(1)?;
                tokens.push(Token::Emphasis { text });
                continue;
            }
        }

        let c = reader.consume_char().unwrap().expect("has to be a char");

        // line breaks
        if c == '\n' {
            if let Ok(Some('\n')) = reader.peek_char(1) {
                // end of paragraph
                break;
            }
            // https://spec.commonmark.org/0.30/#hard-line-break
            if let Some(Token::Text { text }) = tokens.last_mut() {
                println!("{text:?}");
                if text.ends_with("\\") {
                    text.pop();
                    tokens.push(Token::HardBreak);
                    continue;
                }
                if text.ends_with("  ") {
                    *text = text.trim_end().to_string();
                    tokens.push(Token::HardBreak);
                    continue;
                }
            }
            // soft break: https://spec.commonmark.org/0.30/#soft-line-breaks
            tokens.push(Token::SoftBreak);
            continue;
        }
        // push character to text
        if let Some(Token::Text { text }) = tokens.last_mut() {
            text.push(c)
        } else {
            tokens.push(Token::Text { text: c.into() })
        }
    }

    return Ok(tokens);
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Attributes {
        table: toml::map::Map<String, toml::Value>,
    },
    BulletList {
        items: Vec<Vec<Token>>,
    },
    OrderedList {
        items: Vec<Vec<Token>>,
    },
    Heading {
        tokens: Vec<Token>,
        /// 0-6
        depth: u8,
    },
    Html {
        tokens: Vec<Token>,
        tag: String,
        attributes: HashMap<String, String>,
    },
    /// Anything that is not an already declared inline element
    Paragraph {
        tokens: Vec<Token>,
    },
    Bold {
        text: String,
    },
    Emphasis {
        text: String,
    },
    BlockQuote {
        tokens: Vec<Token>,
    },
    Code {
        info: Option<String>,
        text: String,
    },
    /// https://spec.commonmark.org/0.30/#images
    Image {
        /// alt, recommended to convert tokens to text
        tokens: Vec<Token>,
        src: String,
        title: Option<String>,
    },
    Link {
        /// The text portion of a link that contains Tokens
        tokens: Vec<Token>,
        href: String,
        title: Option<String>,
    },
    Text {
        text: String,
    },
    Comment {
        raw: String,
    },
    // https://spec.commonmark.org/0.30/#thematic-breaks
    ThematicBreak,
    HardBreak,
    /// Indicating of a space between paragraphs
    SoftBreak,
}

impl Token {
    pub fn get_tokens(&self) -> Option<Vec<&Token>> {
        match self {
            Token::Heading { tokens, .. }
            | Token::Paragraph { tokens, .. }
            | Token::Link { tokens, .. }
            | Token::Image { tokens, .. }
            | Token::Html { tokens, .. } => Some(tokens.iter().collect()),
            Token::BulletList { items } | Token::OrderedList { items } => {
                let tokens = items.iter().flatten().collect();
                Some(tokens)
            }
            _ => None,
        }
    }

    pub fn to_text(&self) -> Option<String> {
        if let Some(tokens) = self.get_tokens() {
            let mut result = String::new();
            for t in tokens {
                if let Some(text) = t.to_text() {
                    result.push_str(&text)
                }
            }
            return Some(result);
        }
        Some(
            match self {
                Token::Bold { text, .. } => text,
                Token::Text { text, .. } => text,
                Token::SoftBreak { .. } => " ",
                _ => return None,
            }
            .into(),
        )
    }
}
