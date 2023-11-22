use std::{fs::File, io::Read, path::Path};

use crate::{char_reader::CharReader, parse_error::ParseError};

mod lexer;
pub use lexer::*;

pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::new(input);

    let mut tokens = vec![];

    loop {
        match lexer::read_token(&mut reader)? {
            Token::EOF => break,
            t => tokens.push(t),
        }
    }

    // add paragraphs and texts together
    let mut reduced_tokens = vec![];
    for mut token in tokens.into_iter() {
        if let Some(Token::Paragraph { tokens: a }) = reduced_tokens.last_mut() {
            if let Token::Paragraph { tokens: b } = &mut token {
                if let Some(Token::Text { text: text_a }) = a.first_mut() {
                    if let Some(Token::Text { text: text_b }) = b.first_mut() {
                        text_a.push('\n');
                        *text_a += text_b;
                        b.drain(0..1);
                    }
                }
                a.append(b);
                continue;
            }
        }
        reduced_tokens.push(token)
    }

    Ok(reduced_tokens)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Cursor};

    use toml::Table;

    use crate::html::to_attributes;

    use super::*;

    #[test]
    fn test_text_that_looks_like_html() {
        let input = r#"# Rust > c++
Lots of people say Rust > c++. even though it might be
< then c++. Who knows? 
<nonclosing>
This should be text
"#;
        let expected = vec![
            Token::Heading {
                depth: 1,
                tokens: vec![Token::Text {
                    text: "Rust > c++".into(),
                }],
            },
            Token::Paragraph {
                tokens: vec![Token::Text {
                    text: "Lots of people say Rust > c++. even though it might be
< then c++. Who knows?
<nonclosing>
This should be text"
                        .into(),
                }],
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_comments() {
        let input = r#"<!--[default]
title="asdf"
-->
<!-- another comment -->
paragraph <!-- inline comment -->
<!--
another comment
-->
"#;
        let mut attributes_table = Table::new();
        let mut default_table = Table::new();
        default_table.insert("title".into(), "asdf".into());
        attributes_table.insert("default".into(), toml::Value::Table(default_table));
        let expected = vec![
            Token::Attributes {
                toml: toml::Value::Table(attributes_table),
            },
            Token::Comment {
                raw: " another comment ".into(),
            },
            Token::Paragraph {
                tokens: vec![
                    Token::Text {
                        text: "paragraph ".into(),
                    },
                    Token::Comment {
                        raw: " inline comment ".into(),
                    },
                ],
            },
            Token::Comment {
                raw: "\nanother comment\n".into(),
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!( expected,tokens);
    }

    #[test]
    fn test_links() {
        let input = r#"# A [test](test.com)
<div>
<a href="link.com">[other](other.com)</a>
</div>"#;
        let mut attributes_table = Table::new();
        let mut default_table = Table::new();
        default_table.insert("title".into(), "asdf".into());
        attributes_table.insert("default".into(), toml::Value::Table(default_table));
        let expected = vec![
            Token::Heading {
                depth: 1,
                tokens: vec![
                    Token::Text { text: "A ".into() },
                    Token::Link {
                        text: "test".into(),
                        href: "test.com".into(),
                    },
                ],
            },
            Token::Html {
                tag: "div".into(),
                attributes: HashMap::new(),
                tokens: vec![Token::Html {
                    tag: "a".into(),
                    attributes: to_attributes([("href", "link.com")]),
                    tokens: vec![Token::Paragraph {
                        tokens: vec![Token::Link {
                            text: "other".into(),
                            href: "other.com".into(),
                        }],
                    }],
                }],
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!( expected,tokens);
    }
}
