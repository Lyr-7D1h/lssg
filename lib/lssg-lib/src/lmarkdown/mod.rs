use std::io::Read;

use crate::{char_reader::CharReader, parse_error::ParseError};

mod lexer;
pub use lexer::*;

pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::new(input);
    return read_tokens(&mut reader);
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Cursor};

    use toml::Table;

    use crate::dom::to_attributes;

    use super::*;

    fn text(text: &str) -> Token {
        Token::Text { text: text.into() }
    }

    fn p(tokens: Vec<Token>) -> Token {
        Token::Paragraph {
            tokens,
            hard_break: false,
        }
    }

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
            p(vec![
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
            ]),
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
            p(vec![
                Token::Text {
                    text: "paragraph ".into(),
                },
                Token::Comment {
                    raw: " inline comment ".into(),
                },
            ]),
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
[](empty.com)
[<b>bold</b>](bold.com)
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
                        tokens: vec![Token::Text {
                            text: "test".into(),
                        }],
                        href: "test.com".into(),
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
                    },
                    Token::Link {
                        tokens: vec![Token::Html {
                            tag: "b".into(),
                            attributes: HashMap::new(),
                            tokens: vec![Token::Text {
                                text: "bold".into(),
                            }],
                        }],
                        href: "bold.com".into(),
                    },
                    Token::Html {
                        tag: "a".into(),
                        attributes: to_attributes([("href", "link.com")]),
                        tokens: vec![Token::Link {
                            tokens: vec![Token::Text {
                                text: "other".into(),
                            }],
                            href: "other.com".into(),
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
    fn test_setext_heading() {
        let input = r#"Foo *bar*
  ===

Foo *bar*
---------"#;
        let expected = vec![
            Token::Heading {
                tokens: vec![
                    Token::Text {
                        text: "Foo ".into(),
                    },
                    Token::Emphasis { text: "bar".into() },
                ],
                depth: 1,
            },
            Token::Heading {
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
            items: vec![vec![p(vec![text("one"), Token::SoftBreak, text("two")])]],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_bullet_list_indented() {
        let input = r#"- one

  two"#;
        let expected = vec![Token::BulletList {
            items: vec![vec![p(vec![text("one")]), p(vec![text("two")])]],
        }];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_ordered_list() {
        let input = r#"1.  A paragraph
    with two lines.

    > A block quote."#;
        let expected = vec![Token::OrderedList {
            items: vec![vec![
                p(vec![
                    text("A paragraph"),
                    Token::SoftBreak,
                    text("with two lines."),
                ]),
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
        let expected = vec![Token::Code {
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
        let expected = vec![Token::Code {
            text: "a simple\n  indented code block".into(),
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
        let expected = vec![p(vec![
            Token::Code {
                text: "foo".into(),
                info: None,
            },
            Token::SoftBreak,
            Token::Code {
                text: " `` ".into(),
                info: None,
            },
            Token::SoftBreak,
            Token::Code {
                text: "foo ` bar".into(),
                info: None,
            },
        ])];

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
        let expected = vec![p(vec![
            text("foo"),
            Token::HardBreak,
            text("bar"),
            Token::SoftBreak,
            text("foo"),
            Token::HardBreak,
            text("baz"),
        ])];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }

    #[test]
    fn test_autolink() {
        let input = r#"<http://foo.bar.baz>"#;
        let expected = vec![p(vec![Token::Link {
            tokens: vec![text("http://foo.bar.baz")],
            href: "http://foo.bar.baz".into(),
        }])];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(expected, tokens);
    }
}
