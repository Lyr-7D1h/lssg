use std::{collections::HashMap, io::Read};

use log::warn;

use crate::{
    char_reader::CharReader,
    html::{self, element},
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

// official spec: https://spec.commonmark.org/0.30/
// https://github.com/markedjs/marked/blob/master/src/Lexer.ts
// https://github.com/songquanpeng/md2html/blob/main/lexer/lexer.go
// demo: https://marked.js.org/demo/
// demo: https://spec.commonmark.org/dingus/
/// A function to get the next markdown token using recrusive decent.
/// Will first parse a block token (token for a whole line and then parse for any inline tokens when needed.
pub fn read_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {
    let mut tokens = vec![];
    loop {
        match reader.peek_char(0)? {
            None => return Ok(tokens),
            Some(mut c) => {
                // if you start a new block with a newline skip it
                if c == '\n' {
                    reader.consume(0)?;
                    // if more than one newline than could be a blankline
                    let blank_line = reader
                        .consume_until_exclusive(|c| c != '\n' && c != '\r')?
                        .len()
                        > 0;
                    match reader.peek_char(0)? {
                        None => return Ok(tokens),
                        Some(new_c) => c = new_c,
                    }
                    if let Some(token) = read_block_token(c, blank_line, reader, &mut tokens)? {
                        tokens.push(token)
                    }
                    continue;
                }
                if let Some(token) = read_block_token(c, false, reader, &mut tokens)? {
                    tokens.push(token)
                }
            }
        };
    }
}

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
        if let Some((tag, attributes, content)) = element(reader)? {
            let tokens = read_inline_tokens(&content)?;
            return Ok(Some(Token::Html {
                tag,
                attributes,
                tokens,
            }));
        }
    }

    if let Some(heading) = heading(reader)? {
        return Ok(Some(heading));
    }

    if c == '<' {
        // comment
        if let Some(html::Html::Comment { text: raw }) = html::comment(reader)? {
            return Ok(Some(Token::Comment { raw }));
        }

        if let Some((tag, attributes, content)) = html::element(reader)? {
            let mut reader = CharReader::<&[u8]>::from_string(&content);
            let tokens = read_inline_html_tokens(&mut reader)?;
            return Ok(Some(Token::Html {
                tag,
                attributes,
                tokens,
            }));
        }
    }

    // TODO https://spec.commonmark.org/0.30/#setext-heading

    if let Some(tbreak) = thematic_break(reader)? {
        return Ok(Some(tbreak));
    }

    // TODO list

    if let Some(blockquote) = blockquote(reader)? {
        return Ok(Some(blockquote));
    }

    // https://spec.commonmark.org/0.30/#paragraphs
    let text = sanitize_text(reader.consume_until_inclusive(|c| c == '\n')?);
    let mut inline_tokens = read_inline_tokens(&text)?;
    // add to prev p if there isn't a blank line in between
    if let Some(Token::Paragraph {
        tokens: last_tokens,
    }) = tokens.last_mut()
    {
        if !blank_line {
            last_tokens.push(Token::SoftBreak);
            last_tokens.append(&mut inline_tokens);
            return Ok(None);
        }
    }
    return Ok(Some(Token::Paragraph {
        tokens: inline_tokens,
    }));
}

fn read_inline_tokens(text: &String) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::<&[u8]>::from_string(text);

    let mut tokens = vec![];
    while let Some(c) = reader.peek_char(0)? {
        // html
        if c == '<' {
            // comment
            if let Some(html::Html::Comment { text: raw }) = html::comment(&mut reader)? {
                tokens.push(Token::Comment { raw });
                continue;
            }

            if let Some((tag, attributes, content)) = html::element(&mut reader)? {
                let content = sanitize_text(content);
                tokens.push(Token::Html {
                    tag,
                    attributes,
                    tokens: read_inline_tokens(&content)?,
                });
                continue;
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
                            reader.consume(1)?;
                            let alt = read_inline_tokens(&text)?;
                            tokens.push(Token::Image { tokens: alt, src });
                            continue;
                        }
                    }
                }
            }
        }

        // links: https://spec.commonmark.org/0.30/#links
        if c == '[' {
            if let Some(raw_text) = reader.peek_until_inclusive_from(1, |c| c == ']')? {
                let href_start = 1 + raw_text.len();
                if let Some('(') = reader.peek_char(href_start)? {
                    if let Some(raw_href) =
                        reader.peek_until_inclusive_from(href_start + 1, |c| c == ')')?
                    {
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
                    println!("{text}");
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

pub fn thematic_break(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    if let Some(pos) = detect_char_with_ident(|c| c == '*' || c == '-' || c == '_', reader)? {
        let line = reader.peek_line_from(pos)?;
        let line = &line.replace(" ", "")[0..3];
        if line == "***" || line == "---" || line == "___" {
            let a = reader.consume_string(pos + line.len())?;
            return Ok(Some(Token::ThematicBreak));
        }
    }
    return Ok(None);
}

/// ignore up to 4 space idententations returns at which position the match begins
pub fn detect_char_with_ident(
    op: fn(c: char) -> bool,
    reader: &mut CharReader<impl Read>,
) -> Result<Option<usize>, ParseError> {
    for i in 0..4 {
        match reader.peek_char(i)? {
            Some(c) if op(c) => return Ok(Some(i)),
            Some(' ') => {}
            Some(_) | None => return Ok(None),
        }
    }
    return Ok(None);
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
    let mut lines = vec![];
    'outer: loop {
        for i in 0..3 {
            match reader.peek_char(i)? {
                Some('>') => {
                    let line = reader.consume_until_inclusive(|c| c == '\n')?;
                    let text = line[i + 1..line.len() - 1].trim_start().to_string();
                    lines.push(text);
                    continue 'outer;
                }
                Some(' ') => {}
                Some(_) | None => break 'outer,
            }
        }
    }

    if lines.len() == 0 {
        return Ok(None);
    }

    let content = lines.join("\n");

    let mut reader: CharReader<&[u8]> = CharReader::<&[u8]>::from_string(&content);
    reader.set_has_read(true); // prevents attributes
    let tokens = read_tokens(&mut reader)?;
    return Ok(Some(Token::BlockQuote { tokens }));
}

/// https://github.com/markedjs/marked/blob/master/src/Tokenizer.js
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Attributes {
        table: toml::map::Map<String, toml::Value>,
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
    Italic {
        text: String,
    },
    BlockQuote {
        tokens: Vec<Token>,
    },
    Code {
        language: String,
        text: String,
    },
    Image {
        /// alt, recommended to convert tokens to text
        tokens: Vec<Token>,
        src: String,
    },
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
    // https://spec.commonmark.org/0.30/#thematic-breaks
    ThematicBreak,
    HardBreak,
    /// Indicating of a space between paragraphs
    SoftBreak,
}

impl Token {
    pub fn get_tokens(&self) -> Option<&Vec<Token>> {
        match self {
            Token::Heading { tokens, .. }
            | Token::Paragraph { tokens, .. }
            | Token::Link { tokens, .. }
            | Token::Image { tokens, .. }
            | Token::Html { tokens, .. } => Some(tokens),
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
