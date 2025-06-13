use std::io::Read;

use log::warn;
use virtual_dom::Html;

use crate::{char_reader::CharReader, parse_error::ParseError};

use super::{
    html::{html_comment, html_element},
    sanitize_text, Token,
};

/// https://spec.commonmark.org/0.30/#blocks-and-inlines
// FIXME: first parse all block tokens then parse into Token

pub fn read_block_tokens(reader: &mut CharReader<impl Read>) -> Result<Vec<Token>, ParseError> {
    let mut tokens = vec![];
    loop {
        match reader.peek_char(0)? {
            None => return Ok(tokens),
            Some(c) => {
                // if you start a new block with a newline skip it
                if c == '\n' {
                    reader.consume(0)?;
                    // if more than one newline than could be a blankline
                    let blank_line = reader
                        .consume_until_exclusive(|c| c != '\n' && c != '\r')?
                        .len()
                        > 0;
                    if reader.peek_char(0)?.is_none() {
                        return Ok(tokens);
                    }
                    if let Some(token) = from_reader(blank_line, reader, &mut tokens)? {
                        tokens.push(token)
                    }
                    continue;
                }
                if let Some(token) = from_reader(false, reader, &mut tokens)? {
                    tokens.push(token)
                }
            }
        };
    }
}

fn from_reader(
    blank_line: bool,
    reader: &mut CharReader<impl Read>,
    tokens: &mut Vec<Token>,
) -> Result<Option<Token>, ParseError> {
    // if starts with comment in toml format it is an attribute
    if reader.has_read() == false {
        if let Some('<') = reader.peek_char(0)? {
            if reader.peek_string(4)? == "<!--" {
                if let Some(comment) = reader.peek_until_match_inclusive("-->")? {
                    match toml::from_str(&comment[4..comment.len() - 3]) {
                        Ok(toml::Value::Table(table)) => {
                            reader.consume(comment.len())?;
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

    // comment
    if let Some(Html::Comment { text: raw }) = html_comment(reader)? {
        return Ok(Some(Token::Comment { raw }));
    }

    if let Some((tag, attributes, content)) = html_element(reader)? {
        let tokens = if let Some(content) = content {
            let mut reader = CharReader::<&[u8]>::from_string(&content);
            // NOTE: html allows for block tokens inside of it
            read_block_tokens(&mut reader)?
                .into_iter()
                .flat_map(|t| match t {
                    // HACK: dissallowing paragraphs in html so transforming them to inline element Text
                    Token::Paragraph { text, .. } => vec![Token::Text { text }],
                    _ => vec![t],
                })
                .collect()
        } else {
            vec![]
        };

        return Ok(Some(Token::Html {
            tag,
            attributes,
            tokens,
        }));
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

    let text = reader.consume_until_match_inclusive("\n")?;
    if !blank_line {
        if let Some(Token::Paragraph {
            text: last_text, ..
        }) = tokens.last_mut()
        {
            *last_text += &text;
            return Ok(None);
        }
    }
    return Ok(Some(Token::Paragraph {
        text,
        tokens: vec![],
    }));
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

    return Ok(Some(Token::CodeBlock { info: None, text }));
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

        let Some(info) =
            reader.peek_until_inclusive_from(indent + count_backticks, |c| c == '\n')?
        else {
            return Ok(None);
        };
        let info = sanitize_text(info.to_string());
        // info can not contain close fence types
        if info.contains(fence_type) {
            return Ok(None);
        }
        reader.consume(indent + count_backticks + info.len() + 1)?;

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

        return Ok(Some(Token::CodeBlock {
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
    if let Some(Token::Paragraph { text, .. }) = tokens.last() {
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
                    text: text.clone(),
                    tokens: vec![],
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
                    text: text.clone(),
                    tokens: vec![],
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

fn list_item_text(
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
    let mut reader = CharReader::new(item_content.as_bytes());
    let t = read_block_tokens(&mut reader)?;
    Ok(t)
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

        let ident = 1 + pos + n;

        let tokens = list_item_text(reader, ident)?;
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

        let tokens = list_item_text(reader, ident)?;
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

        Ok(Some(Token::Heading {
            depth,
            text,
            tokens: vec![],
        }))
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

    let text = lines.join("\n");
    let mut reader = CharReader::new(text.as_bytes());
    let tokens = read_block_tokens(&mut reader)?;

    return Ok(Some(Token::BlockQuote { tokens }));
}

#[cfg(test)]
mod tests {

    use super::*;

    // fn text(text: &str) -> Token {
    //     Token::Text { text: text.into() }
    // }

    fn p(text: &str) -> Token {
        Token::Paragraph {
            tokens: vec![],
            text: text.into(),
        }
    }

    #[test]
    fn test_block_token() {
        let input = r#"# Rust > c++
Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<nonclosing>
This should be text"#;
        let expected = vec![
            Token::Heading {
                text: "Rust > c++".into(),
                depth: 1,
                tokens: vec![],
            },
            p("Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<nonclosing>
This should be text"),
        ];

        let mut reader = CharReader::new(input.as_bytes());
        let tokens = read_block_tokens(&mut reader).unwrap();
        assert_eq!(expected, tokens);
    }
}
