use std::{collections::HashMap, io, io::Read};

use char_reader::CharReader;

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
    let start_tag_content = start_tag_content.trim();
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
) -> Result<Option<(String, HashMap<String, String>, String)>, io::Error> {
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
    Ok(None)
}

pub fn comment(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, io::Error> {
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
pub fn read_token(reader: &mut CharReader<impl Read>) -> Result<Option<Html>, io::Error> {
    while let Some(c) = reader.peek_char(0)? {
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
        // only valid text if it contains a non whitespace character
        if text.chars().any(|c| c != ' ' && c != '\n') {
            return Ok(Some(Html::Text { text }));
        }
    }

    Ok(None)
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
            Html::Text { text: "\n".into() },
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
}
