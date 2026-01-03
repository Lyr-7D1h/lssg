use std::io::Read;

use log::warn;
use virtual_dom::Html;

use crate::{char_reader::CharReader, parse_error::ParseError};

use super::{
    Token,
    html::{html_comment, html_element},
    sanitize_text,
};

// FIXME: first parse all block tokens then parse into Token
/// https://spec.commonmark.org/0.30/#blocks-and-inlines
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
                    let blank_line = !reader
                        .consume_until_exclusive(|c| c != '\n' && c != '\r')?
                        .is_empty();
                    if reader.peek_char(0)?.is_none() {
                        return Ok(tokens);
                    }
                    if let Some(token) = from_reader(blank_line, c, reader, &mut tokens)? {
                        tokens.push(token)
                    }
                    continue;
                }
                if let Some(token) = from_reader(false, c, reader, &mut tokens)? {
                    tokens.push(token)
                }
            }
        };
    }
}

fn from_reader(
    blank_line: bool,
    c: char,
    reader: &mut CharReader<impl Read>,
    tokens: &mut Vec<Token>,
) -> Result<Option<Token>, ParseError> {
    // if starts with comment in toml format it is an attribute
    if !reader.has_read()
        && let Some('<') = reader.peek_char(0)?
        && reader.peek_string(4)? == "<!--"
        && let Some(comment) = reader.peek_until_match_inclusive("-->")?
    {
        match toml::from_str(&comment[4..comment.len() - 3]) {
            Ok(toml::Value::Table(table)) => {
                reader.consume(comment.len())?;
                return Ok(Some(Token::Attributes { table }));
            }
            Ok(_) => warn!("Attributes is not a table"),
            Err(e) => warn!("Not parsing possible Attributes: {e}"),
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

    if let Some(table) = table(c, reader)? {
        return Ok(Some(table));
    }

    // TODO https://spec.commonmark.org/0.30/#link-reference-definitions

    let text = reader.consume_until_match_inclusive("\n")?;
    if !blank_line
        && let Some(Token::Paragraph {
            text: last_text, ..
        }) = tokens.last_mut()
    {
        *last_text += &text;
        return Ok(None);
    }
    Ok(Some(Token::Paragraph {
        text,
        tokens: vec![],
    }))
}

/// https://spec.commonmark.org/0.30/#indented-code-blocks
pub fn indented_code(
    reader: &mut CharReader<impl Read>,
    tokens: &[Token],
    blank_line: bool,
) -> Result<Option<Token>, ParseError> {
    // can't interupt a paragraph if there wasn't a blank line
    if let Some(Token::Paragraph { .. }) = tokens.last()
        && !blank_line
    {
        return Ok(None);
    }

    let mut text = String::new();
    while "    " == reader.peek_string(4)? {
        let line = reader.consume_until_inclusive(|c| c == '\n')?;
        text.push_str(&line[4..line.len()]);
    }
    if text.is_empty() {
        return Ok(None);
    }

    Ok(Some(Token::CodeBlock { info: None, text }))
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
        if count_backticks < 3 {
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
            if line.is_empty() {
                break;
            }

            let chars: Vec<char> = line.chars().collect();
            // check if closing
            for i in 0..4 {
                match chars.get(i) {
                    Some(c) if *c == fence_type => {
                        // continue if all characters are not same as opening fence
                        for j in i..i + count_backticks {
                            if let Some(c) = chars.get(j)
                                && *c != fence_type
                            {
                                break;
                            }
                        }
                        break 'outer;
                    }
                    Some(' ') => {}
                    Some(_) | None => break,
                }
            }

            let mut pos = 0;
            for c in chars.iter().take(indent) {
                if *c == ' ' {
                    pos += 1;
                    continue;
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

    Ok(None)
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
        if let Some(pattern) = line.replace(" ", "").get(0..3)
            && (pattern == "***" || pattern == "---" || pattern == "___")
        {
            reader.consume_string(pos + line.len())?;
            return Ok(Some(Token::ThematicBreak));
        }
    }
    Ok(None)
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
            if line.is_empty() {
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

    if items.is_empty() {
        return Ok(None);
    }

    Ok(Some(Token::BulletList { items }))
}
// TODO implement all specs (check for same usage of bullet enc.)
/// https://spec.commonmark.org/0.30/#list-items
pub fn ordered_list(reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    let mut items = vec![];
    let mut start = 0;
    while let Some(mut pos) = detect_char_with_ident(reader, |c| c.is_ascii_digit())? {
        let mut closing_char = false;
        for i in 1..10 {
            // not more than 9 digits allowed
            if i == 10 {
                return Ok(None);
            }

            match reader.peek_char(pos + i)? {
                Some(c) if c.is_ascii_digit() => {}
                Some('.') | Some(')') => {
                    pos += i;
                    closing_char = true;
                    break;
                }
                Some(_) | None => return Ok(None),
            }
        }
        if start == 0 {
            start = if closing_char {
                reader.peek_string(pos)?.parse::<u32>().map_err(|e| {
                    ParseError::invalid(format!("Failed to parse start as number: {e}"))
                })?
            } else {
                reader.peek_string(pos + 1)?.parse::<u32>().map_err(|e| {
                    ParseError::invalid(format!("Failed to parse start as number: {e}"))
                })?
            };
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

    if items.is_empty() {
        return Ok(None);
    }

    Ok(Some(Token::OrderedList { items, start }))
}

/// Parse a GitHub Flavored Markdown table
/// https://github.github.com/gfm/#tables-extension-
fn table(c: char, reader: &mut CharReader<impl Read>) -> Result<Option<Token>, ParseError> {
    use super::TableAlign;

    // must start with |
    if c != '|' {
        return Ok(None);
    }

    // Try to parse header row
    let Some(header_line) = reader.peek_until_inclusive(|c| c == '\n')? else {
        return Ok(None);
    };

    // or contain |
    if !header_line.contains('|') {
        return Ok(None);
    }

    // Try to parse delimiter row
    let Some(delimiter_line) =
        reader.peek_until_inclusive_from(header_line.len(), |c| c == '\n')?
    else {
        return Ok(None);
    };

    // Check if delimiter row looks like a table delimiter
    let delimiter_trimmed = delimiter_line.trim();
    if !delimiter_trimmed.starts_with('|') && !delimiter_trimmed.contains('|') {
        return Ok(None);
    }

    // Parse delimiter to check validity and get alignment
    let delimiter_cells: Vec<&str> = delimiter_trimmed
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .collect();

    // Check if delimiter cells are valid (must contain at least one hyphen and only hyphens, colons, and spaces)
    let mut alignments = vec![];
    for cell in &delimiter_cells {
        let trimmed = cell.trim();
        if trimmed.is_empty() || !trimmed.chars().any(|c| c == '-') {
            return Ok(None);
        }
        if !trimmed.chars().all(|c| c == '-' || c == ':' || c == ' ') {
            return Ok(None);
        }

        // Determine alignment
        let starts_with_colon = trimmed.starts_with(':');
        let ends_with_colon = trimmed.ends_with(':');
        let align = match (starts_with_colon, ends_with_colon) {
            (true, true) => TableAlign::Center,
            (true, false) => TableAlign::Left,
            (false, true) => TableAlign::Right,
            (false, false) => TableAlign::None,
        };
        alignments.push(align);
    }

    // Parse header cells
    let header_cells: Vec<&str> = header_line
        .trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(|s| s.trim())
        .collect();

    // Must have at least one column
    if header_cells.is_empty() {
        return Ok(None);
    }

    // Consume header and delimiter
    reader.consume(header_line.len() + delimiter_line.len())?;

    // Parse header tokens
    let header: Vec<Vec<Token>> = header_cells
        .into_iter()
        .map(|cell| {
            vec![Token::Text {
                text: cell.to_string(),
            }]
        })
        .collect();

    // Parse data rows
    let mut rows = vec![];
    while let Some(row_line) = reader.peek_until_inclusive(|c| c == '\n')? {
        let row_trimmed = row_line.trim();

        // Stop if not a table row
        if row_trimmed.is_empty() || (!row_trimmed.starts_with('|') && !row_trimmed.contains('|')) {
            break;
        }

        // Parse row cells
        let row_cells: Vec<&str> = row_trimmed
            .trim_start_matches('|')
            .trim_end_matches('|')
            .split('|')
            .map(|s| s.trim())
            .collect();

        let row: Vec<Vec<Token>> = row_cells
            .into_iter()
            .map(|cell| {
                vec![Token::Text {
                    text: cell.to_string(),
                }]
            })
            .collect();

        rows.push(row);
        reader.consume(row_line.len())?;
    }

    Ok(Some(Token::Table {
        header,
        align: alignments,
        rows,
    }))
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
    Ok(None)
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

    if lines.is_empty() {
        return Ok(None);
    }

    let text = lines.join("\n");
    let mut reader = CharReader::new(text.as_bytes());
    let tokens = read_block_tokens(&mut reader)?;

    Ok(Some(Token::BlockQuote { tokens }))
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

    #[test]
    fn test_table() {
        let input = r#"| Option | Type | Default |
|--------|------|---------|
| `root` | Boolean | `false` |
| `title` | String | First H1 |
"#;

        let mut reader = CharReader::new(input.as_bytes());
        let tokens = read_block_tokens(&mut reader).unwrap();

        assert_eq!(tokens.len(), 1);
        if let Token::Table { header, rows, .. } = &tokens[0] {
            assert_eq!(header.len(), 3); // 3 columns
            assert_eq!(rows.len(), 2); // 2 data rows
        } else {
            panic!("Expected table token, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn test_table_with_alignment() {
        let input = r#"| Left | Center | Right |
|:-----|:------:|------:|
| L1 | C1 | R1 |
"#;

        let mut reader = CharReader::new(input.as_bytes());
        let tokens = read_block_tokens(&mut reader).unwrap();

        assert_eq!(tokens.len(), 1);
        if let Token::Table {
            header,
            align,
            rows,
        } = &tokens[0]
        {
            use crate::lmarkdown::TableAlign;
            assert_eq!(header.len(), 3);
            assert_eq!(align[0], TableAlign::Left);
            assert_eq!(align[1], TableAlign::Center);
            assert_eq!(align[2], TableAlign::Right);
            assert_eq!(rows.len(), 1);
        } else {
            panic!("Expected table token");
        }
    }
}
