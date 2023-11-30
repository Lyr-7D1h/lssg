use std::{collections::HashMap, io::Read};

use crate::{char_reader::CharReader, parse_error::ParseError};

#[macro_export]
macro_rules! html {
    ($x:tt) => {
        $crate::html::parse_html(format!($x).as_bytes())
            .map(|html| match html.into_iter().next() {
                Some(i) => i,
                None => panic!("has to contain valid html"),
            })
            .expect("should contain valid html")
    };
}

pub fn parse_html(input: impl Read) -> Result<Vec<Html>, ParseError> {
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

/// parse html from start to end and return (tag, attributes, innerHtml)
///
/// seperated to make logic more reusable
pub fn html_block(
    reader: &mut CharReader<impl Read>,
) -> Result<Option<(String, HashMap<String, String>, String)>, ParseError> {
    if let Some('<') = reader.peek_char(0)? {
        if let Some(start_tag) = reader.peek_until_exclusive_from(1, |c| c == '>')? {
            // get html tag
            let mut tag = String::new();
            for c in start_tag[1..start_tag.len()].chars() {
                match c {
                    ' ' => break,
                    '\n' => break,
                    _ => tag.push(c),
                }
            }

            // get attributes
            let mut attributes = HashMap::new();
            let chars: Vec<char> = start_tag[1 + tag.len()..start_tag.len() - 1]
                .chars()
                .collect();
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

            let end_tag = format!("</{tag}>");
            if let Some(html_block) = reader.peek_until_match_inclusive(&end_tag)? {
                reader.consume(start_tag.len())?;
                let mut content = reader.consume_string(html_block.len() - start_tag.len())?;
                content.truncate(content.len() - end_tag.len());

                let mut children = vec![];
                let mut reader = CharReader::new(content.as_bytes());
                while let Some(html) = read_token(&mut reader)? {
                    children.push(html);
                }
                return Ok(Some((tag, attributes, content)));
            }
        }
    }
    return Ok(None);
}

pub fn html_comment(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, ParseError> {
    if "<!--" == reader.peek_string(4)? {
        if let Some(text) = reader.peek_until_match_exclusive_from(3, "-->")? {
            reader.consume(4)?; // skip start
            let text = reader.consume_string(text.len())?;
            reader.consume(3)?; // skip end
            return Ok(Some(Html::Comment { text }));
        }
    }

    return Ok(None);
}

/// A "simple" streaming html parser function. This is a fairly simplified way of parsing html
/// ignoring a lot of edge cases and validation normally seen when parsing html.
///
/// **NOTE: Might return multiple Text tokens one after another.**
pub fn read_token(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, ParseError> {
    match reader.peek_char(0)? {
        None => return Ok(None),
        Some(c) => {
            if c == '<' {
                if let Some(comment) = html_comment(reader)? {
                    return Ok(Some(comment));
                }

                if let Some((tag, attributes, content)) = html_block(reader)? {
                    let mut children = vec![];
                    let mut reader = CharReader::new(content.as_bytes());
                    while let Some(html) = read_token(&mut reader)? {
                        children.push(html);
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
            return Ok(Some(Html::Text { text }));
        }
    }
}

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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::html::to_attributes;

    use super::*;

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
            Html::Text { text: "\n".into() },
            Html::Element {
                tag: "button".into(),
                attributes: to_attributes([("disabled", "")]),
                children: vec![],
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_html(reader).unwrap();
        assert_eq!(expected, tokens);

        let input = r#"<div>
<a href="link.com">[other](other.com)</a>
</div>"#;
        let expected = vec![Html::Element {
            tag: "div".into(),
            attributes: HashMap::new(),
            children: vec![
                Html::Text { text: "\n".into() },
                Html::Element {
                    tag: "a".into(),
                    attributes: to_attributes([("href", "link.com")]),
                    children: vec![Html::Text {
                        text: "[other](other.com)".into(),
                    }],
                },
                Html::Text { text: "\n".into() },
            ],
        }];
        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_html(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_text_looks_like_html() {
        let input = r#"<Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<nonclosing>
This should be text
"#;
        let expected = vec![Html::Text {
            text: "<Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<nonclosing>
This should be text
"
            .into(),
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_html(reader).unwrap();
        assert_eq!(expected, tokens);
    }
}
