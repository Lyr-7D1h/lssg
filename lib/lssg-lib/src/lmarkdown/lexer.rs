use std::{collections::HashMap, io::Read, task::Wake};

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
    let text = sanitize_text(reader.consume_until_inclusive(|c| c == '\n')?);
    let mut inline_tokens = read_inline_tokens(&text)?;
    // add to prev p if there isn't a blank line in between
    if let Some(Token::Paragraph {
        tokens: last_tokens,
    }) = tokens.last_mut()
    {
        if !blank_line {
            // https://spec.commonmark.org/0.30/#soft-line-breaks
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
    'outer: while let Some(c) = reader.peek_char(0)? {
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

/// https://spec.commonmark.org/0.30/#indented-code-blocks
pub fn indented_code(
    reader: &mut CharReader<impl Read>,
    tokens: &Vec<Token>,
    blank_line: bool,
) -> Result<Option<Token>, ParseError> {
    // can't interupt a paragraph if there wasn't a blank line
    if let Some(Token::Paragraph { .. }) = tokens.last() {
        if !blank_line {
            return Ok(None);
        }
    }

    let mut text = String::new();
    while "    " == reader.peek_string(4)? {
        let line = reader.consume_until_inclusive(|c| c == '\n')?;
        text.push_str(&line[4..line.len()]);
    }
    if text.len() == 0 {
        return Ok(None);
    }

    return Ok(Some(Token::Code { info: None, text }));
}

/// https://spec.commonmark.org/0.30/#fenced-code-blocks
pub fn fenced_code(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    if let Some(indent) = detect_char_with_ident(reader, |c| c == '~' || c == '`')? {
        let fence_type = reader.peek_char(indent)?.unwrap();
        let mut count_backticks = 1;
        while let Some(c) = reader.peek_char(indent + count_backticks)? {
            if c != fence_type {
                break;
            }
            count_backticks += 1;
        }

        // must start with more than 3 of same fence_type
        if !(count_backticks >= 3) {
            return Ok(None);
        }

        let info = reader.consume_until_inclusive(|c| c == '\n')?;
        let info = &info[indent + count_backticks..info.len()];
        let info = sanitize_text(info.to_string());

        let mut text = String::new();
        // add all content
        'outer: loop {
            let line = reader.consume_until_inclusive(|c| c == '\n')?;
            if line.len() == 0 {
                break;
            }

            let chars: Vec<char> = line.chars().collect();
            // check if closing
            for i in 0..4 {
                match chars.get(i) {
                    Some(c) if *c == fence_type => {
                        // continue if all characters are not same as opening fence
                        for j in i..i + count_backticks {
                            if let Some(c) = chars.get(j) {
                                if *c != fence_type {
                                    break;
                                }
                            }
                        }
                        break 'outer;
                    }
                    Some(' ') => {}
                    Some(_) | None => break,
                }
            }

            let mut pos = 0;
            for i in 0..indent {
                if chars[i] == ' ' {
                    pos += 1;
                }
                break;
            }
            text += &line[pos..line.len()];
        }

        return Ok(Some(Token::Code {
            info: Some(info),
            text,
        }));
    }

    return Ok(None);
}

/// https://spec.commonmark.org/0.30/#setext-heading
pub fn setext_heading(
    reader: &mut CharReader<impl Read>,
    tokens: &mut Vec<Token>,
) -> Result<Option<Token>, ParseError> {
    if let Some(Token::Paragraph { tokens: ptokens }) = tokens.last() {
        if let Some(pos) = detect_char_with_ident(reader, |c| c == '=')? {
            let line = reader.peek_line_from(pos)?;
            if line.len() >= 3 {
                for c in line.chars() {
                    if c != '=' {
                        return Ok(None);
                    }
                }
                reader.consume_string(pos + line.len())?;
                let heading = Token::Heading {
                    tokens: ptokens.clone(),
                    depth: 1,
                };
                tokens.pop(); // remove paragraph
                return Ok(Some(heading));
            }
        } else if let Some(pos) = detect_char_with_ident(reader, |c| c == '-')? {
            let line = reader.peek_line_from(pos)?;
            if line.len() >= 3 {
                for c in line.chars() {
                    if c != '-' {
                        return Ok(None);
                    }
                }
                reader.consume_string(pos + line.len())?;
                let heading = Token::Heading {
                    tokens: ptokens.clone(),
                    depth: 2,
                };
                tokens.pop(); // remove paragraph
                return Ok(Some(heading));
            }
        }
    }
    Ok(None)
}

pub fn thematic_break(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    if let Some(pos) = detect_char_with_ident(reader, |c| c == '*' || c == '-' || c == '_')? {
        let line = reader.peek_line_from(pos)?;
        if let Some(pattern) = line.replace(" ", "").get(0..3) {
            if pattern == "***" || pattern == "---" || pattern == "___" {
                reader.consume_string(pos + line.len())?;
                return Ok(Some(Token::ThematicBreak));
            }
        }
    }
    return Ok(None);
}

fn list_item_tokens(
    reader: &mut CharReader<impl Read>,
    ident: usize,
) -> Result<Vec<Token>, ParseError> {
    // read the first line
    let line = reader.consume_until_inclusive(|c| c == '\n')?;
    let mut item_content = line[ident..line.len()].to_string();
    loop {
        let line = reader.peek_line()?;

        if line.is_empty() {
            let line = reader.consume_string(line.len() + 1)?;
            // end
            if line.len() == 0 {
                break;
            }
            item_content.push_str(&line);
        } else if line.starts_with(&" ".repeat(ident)) {
            let line = reader.consume_string(line.len() + 1)?;
            item_content.push_str(&line[ident..line.len()]);
        } else {
            break;
        }
    }
    let mut reader = CharReader::<&[u8]>::from_string(&item_content);
    return read_tokens(&mut reader);
}
// TODO implement all specs (check for same usage of bullet enc.)
/// https://spec.commonmark.org/0.30/#list-items
pub fn bullet_list(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    let mut items = vec![];

    while let Some(pos) = detect_char_with_ident(reader, |c| c == '-' || c == '+' || c == '*')? {
        // by default n=1
        let mut n = 0;
        for offset in 1..5 {
            match reader.peek_char(pos + offset)? {
                Some(' ') => {}
                Some(_) => {
                    n = offset - 1;
                    break;
                }
                None => return Ok(None),
            }
        }
        // must have atleast one whitespace
        if n == 0 {
            return Ok(None);
        }

        let ident = pos + n;

        let tokens = list_item_tokens(reader, ident)?;
        items.push(tokens)
    }

    if items.len() == 0 {
        return Ok(None);
    }

    return Ok(Some(Token::BulletList { items }));
}
// TODO implement all specs (check for same usage of bullet enc.)
/// https://spec.commonmark.org/0.30/#list-items
pub fn ordered_list(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    let mut items = vec![];
    while let Some(mut pos) = detect_char_with_ident(reader, |c| c.is_ascii_digit())? {
        for i in 1..10 {
            // not more than 9 digits allowed
            if i == 10 {
                return Ok(None);
            }

            match reader.peek_char(pos + i)? {
                Some(c) if c.is_ascii_digit() => {}
                Some('.') | Some(')') => {
                    pos = i + pos;
                    break;
                }
                Some(_) | None => return Ok(None),
            }
        }

        // by default n=1
        let mut n = 1;
        for i in 1..5 {
            match reader.peek_char(pos + i)? {
                Some(' ') => {}
                Some(_) => {
                    n = i;
                    break;
                }
                None => return Ok(None),
            }
        }

        let ident = pos + n;

        let tokens = list_item_tokens(reader, ident)?;
        items.push(tokens)
    }

    if items.len() == 0 {
        return Ok(None);
    }

    return Ok(Some(Token::OrderedList { items }));
}

/// ignore up to 4 space idententations returns at which position the match begins
pub fn detect_char_with_ident(
    reader: &mut CharReader<impl Read>,
    op: fn(c: char) -> bool,
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
    if let Some(pos) = detect_char_with_ident(reader, |c| c == '#')? {
        let chars: Vec<char> = reader.peek_string_from(pos, 7)?.chars().collect();
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
                    let text = line[i + 1..line.len()].trim_start().to_string();
                    lines.push(text);
                    continue 'outer;
                }
                Some(' ') => {}
                Some(_) | None => break 'outer,
            }
        }
        break;
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
            // TODO lists
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
