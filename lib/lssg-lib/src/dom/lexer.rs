use std::{
    collections::{HashMap, VecDeque},
    io::Read,
};

use crate::{char_reader::CharReader, parse_error::ParseError};

use super::DomNode;

#[macro_export]
macro_rules! html {
    ($x:tt) => {
        $crate::dom::parse_html(format!($x).as_bytes())
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

/// parse html from start to end and return (tag, attributes, innerHtml)
///
/// seperated to make logic more reusable
pub fn element(
    reader: &mut CharReader<impl Read>,
) -> Result<Option<(String, HashMap<String, String>, String)>, ParseError> {
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

            let end_tag = format!("</{tag}>");
            if let Some(html_block) =
                reader.peek_until_match_exclusive_from(2 + start_tag.len(), &end_tag)?
            {
                // <{start_tag}>
                reader.consume(start_tag.len() + 2)?;

                let attributes = attributes(&start_tag[tag.len()..start_tag.len()])?;

                let content = reader.consume_string(html_block.len())?;
                reader.consume(end_tag.len())?;

                return Ok(Some((tag, attributes, content)));
            }
        }
    }
    return Ok(None);
}

pub fn comment(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, ParseError> {
    if "<!--" == reader.peek_string(4)? {
        if let Some(text) = reader.peek_until_match_exclusive_from(4, "-->")? {
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
                if let Some(comment) = comment(reader)? {
                    return Ok(Some(comment));
                }

                if let Some((tag, attributes, content)) = element(reader)? {
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

impl Into<DomNode> for Html {
    fn into(self) -> DomNode {
        match self {
            Html::Comment { .. } => panic!("root html can't be comment"),
            Html::Text { text } => DomNode::create_text(text),
            Html::Element {
                tag,
                attributes,
                children,
            } => {
                let root = DomNode::create_element_with_attributes(tag, attributes);
                let mut queue: VecDeque<(Html, DomNode)> = VecDeque::from(
                    children
                        .into_iter()
                        .zip(std::iter::repeat(root.clone()))
                        .collect::<Vec<(Html, DomNode)>>(),
                );
                while let Some((c, parent)) = queue.pop_front() {
                    if let Some(p) = match c {
                        Html::Text { text } => Some(DomNode::create_text(text)),
                        Html::Element {
                            tag,
                            attributes,
                            children,
                        } => {
                            let p = DomNode::create_element_with_attributes(tag, attributes);
                            queue.extend(children.into_iter().zip(std::iter::repeat(p.clone())));
                            Some(p)
                        }
                        _ => None,
                    } {
                        parent.append_child(p)
                    }
                }
                root
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::dom::to_attributes;

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

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_html(reader).unwrap();
        assert_eq!(expected, tokens);
    }
}
