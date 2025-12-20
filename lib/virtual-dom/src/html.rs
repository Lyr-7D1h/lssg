use std::{collections::HashMap, io, io::Read};

use char_reader::CharReader;

use crate::DomNode;

pub fn parse_html_from_string(input: &String) -> Result<Vec<Html>, io::Error> {
    return parse_html(input.as_bytes());
}

// TODO: return DomNode directly instead of parsing to intermediary representation
pub fn parse_html(input: impl Read) -> Result<Vec<Html>, io::Error> {
    let mut reader = CharReader::new(input);

    let mut tokens = vec![];

    loop {
        match read_token(&mut reader)? {
            None => break,
            Some(t) => tokens.push(t),
        }
    }

    // add texts together
    let mut reduced_tokens = vec![];
    for token in tokens.into_iter() {
        if let Some(Html::Text { text: a }) = reduced_tokens.last_mut() {
            if let Html::Text { text: b } = &token {
                *a += b;
                continue;
            }
        }
        reduced_tokens.push(token)
    }

    Ok(reduced_tokens)
}

fn attributes(start_tag_content: &str) -> Result<HashMap<String, String>, io::Error> {
    // remove whitespace before and after text
    let start_tag_content = start_tag_content.trim();
    let chars: Vec<char> = start_tag_content.chars().collect();
    let mut attributes = HashMap::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\n' if in_value == false => {
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
                _ => {
                    // '=' not followed by a quote
                    if in_value {
                        value.push('=')
                    } else {
                        key.push('=')
                    }
                }
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

/// Get the start tag with its attributes starts after the opening tag '<'
///
/// returns (tag, attributes, tag_content_length, void_element)
fn element_start_tag(
    reader: &mut CharReader<impl Read>,
) -> Result<Option<(String, HashMap<String, String>, usize, bool)>, io::Error> {
    let mut inside_single_quotes = false;
    let mut inside_double_quotes = false;
    let mut i = 1;
    while let Some(c) = reader.peek_char(i)? {
        match c {
            '>' if inside_single_quotes == false && inside_double_quotes == false => {
                let tag_content = reader.peek_string(i + 1)?;

                let mut tag = String::new();
                for c in tag_content.chars().skip(1) {
                    match c {
                        ' ' | '\n' | '>' | '/' => break,
                        _ => tag.push(c),
                    }
                }

                let void_element = reader.peek_char(i - 1)? == Some('/') && is_void_element(&tag);
                // if it is void it is one character less (exclude the / and >)
                // otherwise just exclude the >
                let attributes_end = if void_element {
                    tag_content.len() - 2
                } else {
                    tag_content.len() - 1
                };

                let attributes = attributes(&tag_content[tag.len() + 1..attributes_end])?;

                return Ok(Some((tag, attributes, i + 1, void_element)));
            }
            '"' if !inside_single_quotes => inside_double_quotes = !inside_double_quotes,
            '\'' if !inside_double_quotes => inside_single_quotes = !inside_single_quotes,
            _ => {}
        }
        i += 1;
    }
    Ok(None)
}

/// Find the matching closing tag while respecting nesting
fn find_matching_closing_tag(
    reader: &mut CharReader<impl Read>,
    tag: &str,
    start_offset: usize,
) -> Result<Option<usize>, io::Error> {
    let start_tag = format!("<{}", tag);
    let end_tag = format!("</{}>", tag);
    let mut depth = 0;
    let mut i = start_offset;
    let mut in_double_quotes = false;
    let mut in_single_quotes = false;

    loop {
        // Try to peek ahead to see if we have more content
        let peek_char = reader.peek_char(i)?;
        if peek_char.is_none() {
            return Ok(None);
        }

        let current_char = peek_char.unwrap();

        // Track quote state to ignore tags inside attribute values
        match current_char {
            '"' if !in_single_quotes => in_double_quotes = !in_double_quotes,
            '\'' if !in_double_quotes => in_single_quotes = !in_single_quotes,
            _ => {}
        }

        // Only look for tags when not inside quotes
        if !in_double_quotes && !in_single_quotes && current_char == '<' {
            // Check if we can match the start tag at position i
            let start_tag_len = start_tag.len();
            if let Ok(peek_start) = reader.peek_string_from(i, start_tag_len + 1) {
                if peek_start.starts_with(&start_tag) {
                    // Make sure it's actually a tag (followed by space, >, or /)
                    if let Some(next_char) = peek_start.chars().nth(start_tag_len) {
                        if next_char == ' ' || next_char == '>' || next_char == '/' {
                            depth += 1;
                            i += start_tag_len;
                            continue;
                        }
                    }
                }
            }

            // Check if we can match the end tag at position i
            let end_tag_len = end_tag.len();
            if let Ok(peek_end) = reader.peek_string_from(i, end_tag_len) {
                if peek_end == end_tag {
                    if depth == 0 {
                        return Ok(Some(i - start_offset));
                    }
                    depth -= 1;
                    i += end_tag_len;
                    continue;
                }
            }
        }

        i += 1;
    }
}

/// parse html from start to end and return (tag, attributes, innerHtml)
///
/// seperated to make logic more reusable
fn element(
    reader: &mut CharReader<impl Read>,
) -> Result<Option<(String, HashMap<String, String>, Option<String>)>, io::Error> {
    if let Some('<') = reader.peek_char(0)? {
        if let Some((tag, attributes, tag_content_length, void_element)) =
            element_start_tag(reader)?
        {
            // <{start_tag}/>
            if void_element {
                reader.consume(tag_content_length)?;
                return Ok(Some((tag, attributes, None)));
            }

            // <{start_tag}>{content}</{start_tag}>
            if let Some(content_length) =
                find_matching_closing_tag(reader, &tag, tag_content_length)?
            {
                reader.consume(tag_content_length)?;
                let content = reader.consume_string(content_length)?;
                reader.consume(tag.len() + 3)?; // </{tag}>

                return Ok(Some((tag, attributes, Some(content))));
            }
        }
    }
    Ok(None)
}

fn comment(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, io::Error> {
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
pub fn is_void_element(tag: &str) -> bool {
    match tag {
        "base" | "img" | "br" | "col" | "embed" | "hr" | "area" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => true,
        _ => false,
    }
}

/// A "simple" streaming html parser function. This is a fairly simplified way of parsing html
/// ignoring a lot of edge cases and validation normally seen when parsing html.
///
/// **NOTE: Might return multiple Text tokens one after another.**
fn read_token(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, io::Error> {
    while let Some(c) = reader.peek_char(0)? {
        if c == '<' {
            if let Some(comment) = comment(reader)? {
                return Ok(Some(comment));
            }

            if let Some((tag, attributes, content)) = element(reader)? {
                let mut children = vec![];
                if let Some(content) = content {
                    let mut reader = CharReader::new(content.as_bytes());
                    while let Some(html) = read_token(&mut reader)? {
                        children.push(html);
                    }
                }
                return Ok(Some(Html::Element {
                    tag,
                    attributes,
                    children,
                }));
            }

            // non html opening
            reader.consume(1)?;
            let mut text = "<".to_string();
            text.push_str(&reader.consume_until_exclusive(|c| c == '<')?);
            return Ok(Some(Html::Text { text }));
        }

        let text = reader.consume_until_exclusive(|c| c == '<')?;
        // only valid text if it contains a non whitespace character
        if text.chars().any(|c| c != ' ' && c != '\n') {
            return Ok(Some(Html::Text { text }));
        }
    }

    Ok(None)
}

/// Simple parsed html representation with recursively added children
#[derive(Debug, Clone, PartialEq)]
pub enum Html {
    Comment {
        text: String,
    },
    Text {
        text: String,
    },
    Element {
        tag: String,
        attributes: HashMap<String, String>,
        children: Vec<Html>,
    },
}

impl From<DomNode> for Html {
    fn from(value: DomNode) -> Self {
        match &*value.kind() {
            crate::DomNodeKind::Text { text } => Html::Text { text: text.clone() },
            crate::DomNodeKind::Element { tag, attributes } => {
                let children = value.children().into_iter().map(|c| c.into()).collect();
                Html::Element {
                    tag: tag.clone(),
                    attributes: attributes.clone(),
                    children,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Utility function to convert iteratables into attributes hashmap
    pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
        arr: I,
    ) -> HashMap<String, String> {
        arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
    }

    #[test]
    fn test_html() {
        let input = r#"<a href="test.com"><i class="fa-solid fa-rss"></i>Test</a>
<button disabled></button>"#;
        let expected = vec![
            Html::Element {
                tag: "a".into(),
                attributes: to_attributes([("href", "test.com")]),
                children: vec![
                    Html::Element {
                        tag: "i".into(),
                        attributes: to_attributes([("class", "fa-solid fa-rss")]),
                        children: vec![],
                    },
                    Html::Text {
                        text: "Test".into(),
                    },
                ],
            },
            Html::Element {
                tag: "button".into(),
                attributes: to_attributes([("disabled", "")]),
                children: vec![],
            },
        ];

        let tokens = parse_html(input.as_bytes()).unwrap();
        assert_eq!(expected, tokens);

        let input = r#"<div>
<a href="link.com">[other](other.com)</a>
</div>"#;
        let expected = vec![Html::Element {
            tag: "div".into(),
            attributes: HashMap::new(),
            children: vec![Html::Element {
                tag: "a".into(),
                attributes: to_attributes([("href", "link.com")]),
                children: vec![Html::Text {
                    text: "[other](other.com)".into(),
                }],
            }],
        }];
        let tokens = parse_html(input.as_bytes()).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_text_looks_like_html() {
        let input = r#"<Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<>
<nonclosing>
This should be text
"#;
        let expected = vec![Html::Text {
            text: "<Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<>
<nonclosing>
This should be text
"
            .into(),
        }];

        let tokens = parse_html(input.as_bytes()).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_js_in_attribute() {
        let input = r#"<div onclick="() => test()"></div>"#;

        let expected = vec![Html::Element {
            tag: "div".into(),
            attributes: to_attributes([("onclick", "() => test()")]),
            children: vec![],
        }];
        let tokens = parse_html(input.as_bytes()).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_nested_elements() {
        let input = r#"<div class="a">
            <div class="b">
                <div class="c">
                </div>
            </div>
        </div>
        "#;
        let expected = vec![Html::Element {
            tag: "div".into(),
            attributes: to_attributes([("class", "a")]),
            children: vec![Html::Element {
                tag: "div".into(),
                attributes: to_attributes([("class", "b")]),
                children: vec![Html::Element {
                    tag: "div".into(),
                    attributes: to_attributes([("class", "c")]),
                    children: vec![],
                }],
            }],
        }];
        let tokens = parse_html(input.as_bytes()).unwrap();
        assert_eq!(expected, tokens);
    }
}
