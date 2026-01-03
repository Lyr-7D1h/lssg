use std::{collections::HashMap, io::Read};

use virtual_dom::Html;

use crate::{char_reader::CharReader, parse_error::ParseError};

/// from virtual_dom::html
fn attributes(start_tag_content: &str) -> Result<HashMap<String, String>, ParseError> {
    let chars: Vec<char> = start_tag_content.chars().collect();
    let mut attributes = HashMap::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' if in_value == false => {
                if key.len() > 0 {
                    attributes.insert(key, value);
                    key = String::new();
                    value = String::new();
                    in_value = false;
                }
            }
            '=' => match chars.get(i + 1) {
                Some('"') | Some('\'') => {
                    i += 1;
                    in_value = true
                }
                _ => {}
            },
            '\'' | '"' if in_value => in_value = false,
            c => {
                if in_value {
                    value.push(c)
                } else {
                    key.push(c)
                }
            }
        }
        i += 1;
    }
    if key.len() > 0 {
        attributes.insert(key, value);
    }

    Ok(attributes)
}

/// from virtual_dom::html
pub fn html_element(
    reader: &mut CharReader<impl Read>,
) -> Result<Option<(String, HashMap<String, String>, Option<String>)>, ParseError> {
    if let Some('<') = reader.peek_char(0)? {
        if let Some(start_tag) = reader.peek_until_exclusive_from(1, |c| c == '>')? {
            // get html tag
            let mut tag = String::new();
            for c in start_tag.chars() {
                match c {
                    ' ' => break,
                    '\n' => break,
                    _ => tag.push(c),
                }
            }

            if start_tag.ends_with("/") && is_void_element(&tag) {
                // <{start_tag}/>
                reader.consume(start_tag.len() + 2)?;
                let attributes = attributes(&start_tag[tag.len()..start_tag.len() - 1])?;
                return Ok(Some((tag, attributes, None)));
            }

            let end_tag = format!("</{tag}>");
            if let Some(html_block) =
                reader.peek_until_match_exclusive_from(2 + start_tag.len(), &end_tag)?
            {
                // <{start_tag}>
                reader.consume(start_tag.len() + 2)?;

                let attributes = attributes(&start_tag[tag.len()..start_tag.len()])?;

                let content = reader.consume_string(html_block.len())?;
                reader.consume(end_tag.len())?;

                return Ok(Some((tag, attributes, Some(content))));
            }
        }
    }
    Ok(None)
}

/// from virtual_dom::html
pub fn html_comment(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, ParseError> {
    if "<!--" == reader.peek_string(4)? {
        if let Some(text) = reader.peek_until_match_exclusive_from(4, "-->")? {
            reader.consume(4)?; // skip start
            let text = reader.consume_string(text.len())?;
            reader.consume(3)?; // skip end
            return Ok(Some(Html::Comment { text }));
        }
    }

    Ok(None)
}

/// check if a html tag is a void tag (it can not have children)
fn is_void_element(tag: &str) -> bool {
    match tag {
        // HTML void elements
        "base" | "img" | "br" | "col" | "embed" | "hr" | "area" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" 
        // SVG void-like elements
        | "circle" | "ellipse" | "line" | "path" | "polygon" | "polyline" | "rect" 
        | "stop" | "use" => true,
        _ => false,
    }
}
