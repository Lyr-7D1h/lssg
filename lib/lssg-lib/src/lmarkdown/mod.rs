use std::io::Read;

use crate::{char_reader::CharReader, parse_error::ParseError};

mod block_token;
mod html;
mod inline_token;
mod lexer;
mod tokenizer;
pub use lexer::*;

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

/// Parse LMarkdown using a recursive decent parser
///
/// **NOTE: Current implementation is fairly wonky but fast**
pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::new(input);
    return read_tokens(&mut reader);
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Cursor, io::Read};

    use toml::{Table, Value};

    use super::{parse_lmarkdown, Token};

    /// Utility function to convert iteratables into attributes hashmap
    fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
        arr: I,
    ) -> HashMap<String, String> {
        arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
    }

    fn text(text: &str) -> Token {
        Token::Text { text: text.into() }
    }

    fn p(tokens: Vec<Token>) -> Token {
        let lines: Vec<String> = tokens.iter().filter_map(|t| t.to_text()).collect();
        Token::Paragraph {
            tokens,
            text: lines.join(""),
        }
    }

    #[test]
    fn test_text_that_looks_like_html() {
        let input = r#"# Rust > c++
Lots of people say Rust > c++. even though it might be
< then c++. Who knows?
<nonclosing>
This should be text"#;
        let expected = vec![
            Token::Heading {
                depth: 1,
                text: "Rust > c++".into(),
                tokens: vec![Token::Text {
                    text: "Rust > c++".into(),
                }],
            },
            Token::Paragraph {
                tokens: vec![
                    Token::Text {
                        text: "Lots of people say Rust > c++. even though it might be".into(),
                    },
                    Token::SoftBreak,
                    Token::Text {
                        text: "< then c++. Who knows?".into(),
                    },
                    Token::SoftBreak,
                    Token::Text {
                        text: "<nonclosing>".into(),
                    },
                    Token::SoftBreak,
                    Token::Text {
                        text: "This should be text".into(),
                    },
                ],
                text: "Lots of people say Rust > c++. even though it might be
< then c++. Who knows?
<nonclosing>
This should be text"
                    .into(),
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
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
                table: attributes_table,
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
                text: String::from("paragraph <!-- inline comment -->\n"),
            },
            Token::Comment {
                raw: "\nanother comment\n".into(),
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_links() {
        let input = r#"# A [test](test.com)
<div>
[](empty.com "Empty")
[<b>bold</b>](bold.com)
<a href="link.com">[other](other.com)</a>
</div>"#;
        let mut attributes_table = Table::new();
        let mut default_table = Table::new();
        default_table.insert("title".into(), "asdf".into());
        attributes_table.insert("default".into(), toml::Value::Table(default_table));
        let expected = vec![
            Token::Heading {
                text: "A [test](test.com)".into(),
                depth: 1,
                tokens: vec![
                    Token::Text { text: "A ".into() },
                    Token::Link {
                        tokens: vec![Token::Text {
                            text: "test".into(),
                        }],
                        href: "test.com".into(),
                        title: None,
                    },
                ],
            },
            Token::Html {
                tag: "div".into(),
                attributes: HashMap::new(),
                tokens: vec![
                    Token::Link {
                        tokens: vec![],
                        href: "empty.com".into(),
                        title: Some("Empty".into()),
                    },
                    Token::SoftBreak,
                    Token::Link {
                        tokens: vec![Token::Html {
                            tag: "b".into(),
                            attributes: HashMap::new(),
                            tokens: vec![Token::Text {
                                text: "bold".into(),
                            }],
                        }],
                        href: "bold.com".into(),
                        title: None,
                    },
                    Token::Html {
                        tag: "a".into(),
                        attributes: to_attributes([("href", "link.com")]),
                        tokens: vec![Token::Link {
                            tokens: vec![Token::Text {
                                text: "other".into(),
                            }],
                            href: "other.com".into(),
                            title: None,
                        }],
                    },
                ],
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_inline_in_heading() {
        let input = r#"# foo *bar*"#;
        let expected = vec![Token::Heading {
            text: "foo *bar*".into(),
            tokens: vec![
                Token::Text {
                    text: "foo ".into(),
                },
                Token::Emphasis { text: "bar".into() },
            ],
            depth: 1,
        }];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_setext_heading() {
        let input = r#"Foo *bar*
===

Foo *bar*
---------"#;
        let expected = vec![
            Token::Heading {
                text: "Foo *bar*\n".into(),
                tokens: vec![
                    Token::Text {
                        text: "Foo ".into(),
                    },
                    Token::Emphasis { text: "bar".into() },
                ],
                depth: 1,
            },
            Token::Heading {
                text: "Foo *bar*\n".into(),
                tokens: vec![
                    Token::Text {
                        text: "Foo ".into(),
                    },
                    Token::Emphasis { text: "bar".into() },
                ],
                depth: 2,
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_bullet_list() {
        let input = r#"- one
  two
"#;
        let expected = vec![Token::BulletList {
            items: vec![vec![Token::Paragraph {
                tokens: vec![text("one"), Token::SoftBreak, text("two")],
                text: "one\ntwo\n".into(),
            }]],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    // #[test]
    //     fn test_bullet_list_indented() {
    //         let input = r#"- one

    //   two"#;
    //         let expected = vec![Token::BulletList {
    //             items: vec![vec![
    //                 Token::Paragraph {
    //                     text: "one\n".into(),
    //                     tokens: vec![text("one")],
    //                 },
    //                 Token::Paragraph {
    //                     text: "two".into(),
    //                     tokens: vec![text("two")],
    //                 },
    //             ]],
    //         }];

    //         let reader: Box<dyn Read> = Box::new(Cursor::new(input));
    //         let tokens = parse_lmarkdown(reader).unwrap();
    //         assert_eq!(expected, tokens);
    //     }
    #[test]
    fn test_ordered_list() {
        let input = r#"
1.  A paragraph
    with two lines.

    > A block quote."#;
        let expected = vec![Token::OrderedList {
            start: 1,
            items: vec![vec![
                Token::Paragraph {
                    tokens: vec![
                        text("A paragraph"),
                        Token::SoftBreak,
                        text("with two lines."),
                    ],
                    text: "A paragraph\nwith two lines.\n".into(),
                },
                Token::BlockQuote {
                    tokens: vec![p(vec![text("A block quote.")])],
                },
            ]],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_code_fenced() {
        let input = r#"```markdown
aaa
~~~
```"#;
        let expected = vec![Token::CodeBlock {
            info: Some("markdown".into()),
            text: "aaa\n~~~\n".into(),
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_indented_code() {
        let input = r#"    a simple
    indented code block"#;
        let expected = vec![Token::CodeBlock {
            text: "a simple
indented code block"
                .into(),
            info: None,
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_code_span() {
        let input = r#"`foo`
` `` `
`` foo ` bar ``"#;
        let expected = vec![Token::Paragraph {
            text: "`foo`\n` `` `\n`` foo ` bar ``".into(),
            tokens: vec![
                Token::Code { text: "foo".into() },
                Token::SoftBreak,
                Token::Code {
                    text: " `` ".into(),
                },
                Token::SoftBreak,
                Token::Code {
                    text: "foo ` bar".into(),
                },
            ],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_hard_line_break() {
        let input = r#"foo  
bar
foo\
baz"#;
        let expected = vec![Token::Paragraph {
            text: "foo  \nbar\nfoo\\\nbaz".into(),
            tokens: vec![
                text("foo"),
                Token::HardBreak,
                text("bar"),
                Token::SoftBreak,
                text("foo"),
                Token::HardBreak,
                text("baz"),
            ],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_autolink() {
        let input = r#"<http://foo.bar.baz>"#;
        let expected = vec![Token::Paragraph {
            tokens: vec![Token::Link {
                tokens: vec![text("http://foo.bar.baz")],
                href: "http://foo.bar.baz".into(),
                title: None,
            }],
            text: r#"<http://foo.bar.baz>"#.into(),
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_html_in_comments() {
        let input = r#"<!--<test></test>-->"#;
        let expected = vec![Token::Comment {
            raw: "<test></test>".into(),
        }];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_html_in_attributes() {
        let input = r#"<!--
test='<test></test>'
-->"#;
        let expected = vec![Token::Attributes {
            table: [("test".to_string(), Value::String("<test></test>".into()))]
                .into_iter()
                .collect(),
        }];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }
}
