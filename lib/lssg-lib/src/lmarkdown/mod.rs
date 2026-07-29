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
        if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }

    lines.join("\n")
}

/// Parse LMarkdown using a recursive decent parser
///
/// **NOTE: Current implementation is fairly wonky but fast**
pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut reader = CharReader::new(input);
    read_tokens(&mut reader)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::Cursor, io::Read};

    use toml::{Table, Value};

    use super::{Token, parse_lmarkdown};

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
                Token::Emphasis {
                    text: "bar".into(),
                    tokens: vec![Token::Text { text: "bar".into() }],
                },
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
                    Token::Emphasis {
                        text: "bar".into(),
                        tokens: vec![Token::Text { text: "bar".into() }],
                    },
                ],
                depth: 1,
            },
            Token::Heading {
                text: "Foo *bar*\n".into(),
                tokens: vec![
                    Token::Text {
                        text: "Foo ".into(),
                    },
                    Token::Emphasis {
                        text: "bar".into(),
                        tokens: vec![Token::Text { text: "bar".into() }],
                    },
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
    fn test_code_no_language() {
        let input = r#"```
code block without language
```"#;
        let expected = vec![Token::CodeBlock {
            text: "code block without language\n".into(),
            info: None,
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

    // GFM Autolinks Extension Tests
    // https://github.github.com/gfm/#autolinks-extension-
    #[test]
    fn test_autolink_www() {
        // Example 622
        let input = "www.commonmark.org";
        let expected = vec![p(vec![Token::Autolink {
            href: "http://www.commonmark.org".into(),
            text: "www.commonmark.org".into(),
        }])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_www_with_path() {
        // Example 623
        let input = "Visit www.commonmark.org/help for more information.";
        let expected = vec![p(vec![
            text("Visit "),
            Token::Autolink {
                href: "http://www.commonmark.org/help".into(),
                text: "www.commonmark.org/help".into(),
            },
            text(" for more information."),
        ])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_www_trailing_punctuation() {
        // Example 624
        let input = r#"Visit www.commonmark.org.
Visit www.commonmark.org/a.b."#;
        let expected = vec![Token::Paragraph {
            text: "Visit www.commonmark.org.\nVisit www.commonmark.org/a.b.".into(),
            tokens: vec![
                text("Visit "),
                Token::Autolink {
                    href: "http://www.commonmark.org".into(),
                    text: "www.commonmark.org".into(),
                },
                text("."),
                Token::SoftBreak,
                text("Visit "),
                Token::Autolink {
                    href: "http://www.commonmark.org/a.b".into(),
                    text: "www.commonmark.org/a.b".into(),
                },
                text("."),
            ],
        }];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_http() {
        // Example 629 - modified to match standard markdown behavior
        // Single newline creates softbreak, not paragraph boundary
        let input = r#"http://commonmark.org
(Visit https://encrypted.google.com/search?q=Markup+(business))"#;
        let expected = vec![Token::Paragraph {
            text: "http://commonmark.org\n(Visit https://encrypted.google.com/search?q=Markup+(business))".into(),
            tokens: vec![
                Token::Autolink {
                    href: "http://commonmark.org".into(),
                    text: "http://commonmark.org".into(),
                },
                Token::SoftBreak,
                text("(Visit "),
                Token::Autolink {
                    href: "https://encrypted.google.com/search?q=Markup+(business)".into(),
                    text: "https://encrypted.google.com/search?q=Markup+(business)".into(),
                },
                text(")"),
            ],
        }];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_email() {
        // Example 630
        let input = "foo@bar.baz";
        let expected = vec![p(vec![Token::Autolink {
            href: "mailto:foo@bar.baz".into(),
            text: "foo@bar.baz".into(),
        }])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_email_with_plus() {
        // Example 631
        let input = "hello@mail+xyz.example isn't valid, but hello+xyz@mail.example is.";
        let expected = vec![p(vec![
            text("hello@mail+xyz.example isn't valid, but "),
            Token::Autolink {
                href: "mailto:hello+xyz@mail.example".into(),
                text: "hello+xyz@mail.example".into(),
            },
            text(" is."),
        ])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_not_in_angle_brackets() {
        // Example 620 - http without angle brackets should autolink with extension
        let input = "http://example.com";
        let expected = vec![p(vec![Token::Autolink {
            href: "http://example.com".into(),
            text: "http://example.com".into(),
        }])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_autolink_email_not_in_angle_brackets() {
        // Example 621 - email without angle brackets should autolink with extension
        let input = "foo@bar.example.com";
        let expected = vec![p(vec![Token::Autolink {
            href: "mailto:foo@bar.example.com".into(),
            text: "foo@bar.example.com".into(),
        }])];
        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_link_in_image_text_works() {
        let input = "![My [website](https://example.com)](example.com/image)";
        let expected = vec![Token::Paragraph {
            text: input.into(),
            tokens: vec![Token::Image {
                tokens: vec![
                    text("My "),
                    Token::Link {
                        tokens: vec![text("website")],
                        href: "https://example.com".into(),
                        title: None,
                    },
                ],
                src: "example.com/image".into(),
                title: None,
            }],
        }];

        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_parantheses_in_link_works() {
        let input = "[Glob](https://en.wikipedia.org/wiki/Glob_(programming))";
        let expected = vec![Token::Paragraph {
            text: input.to_string(),
            tokens: vec![Token::Link {
                tokens: vec![text("Glob")],
                href: "https://en.wikipedia.org/wiki/Glob_(programming)".to_string(),
                title: None,
            }],
        }];

        let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
        assert_eq!(tokens, expected);
    }

    mod table_tests {
        use crate::lmarkdown::{TableAlign, Token, parse_lmarkdown};

        fn text(text: &str) -> Token {
            Token::Text { text: text.into() }
        }

        #[test]
        fn test_table_simple() {
            // GFM Example 198: Simple table
            let input = "| foo | bar |\n| --- | --- |\n| baz | bim |\n";
            let expected = vec![Token::Table {
                header: vec![vec![text("foo")], vec![text("bar")]],
                align: vec![TableAlign::None, TableAlign::None],
                rows: vec![vec![vec![text("baz")], vec![text("bim")]]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_alignment() {
            // GFM Example 199: Table with left/right alignment
            let input = "| abc | defghi |\n| :--- | ---: |\n| bar | baz |\n";
            let expected = vec![Token::Table {
                header: vec![vec![text("abc")], vec![text("defghi")]],
                align: vec![TableAlign::Left, TableAlign::Right],
                rows: vec![vec![vec![text("bar")], vec![text("baz")]]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_center_alignment() {
            let input = "| abc | def |\n| :---: | :---: |\n| bar | baz |\n";
            let expected = vec![Token::Table {
                header: vec![vec![text("abc")], vec![text("def")]],
                align: vec![TableAlign::Center, TableAlign::Center],
                rows: vec![vec![vec![text("bar")], vec![text("baz")]]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_uneven_cells() {
            // GFM Example 200: Rows with different cell counts
            let input = "| abc | def |\n| --- | --- |\n| bar |\n| bar | baz | boo |\n";
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();

            assert_eq!(tokens.len(), 1);
            if let Token::Table {
                header,
                align,
                rows,
            } = &tokens[0]
            {
                assert_eq!(header.len(), 2);
                assert_eq!(align.len(), 2);
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 1); // first row has 1 cell
                assert_eq!(rows[1].len(), 3); // second row has 3 cells
            } else {
                panic!("Expected table token");
            }
        }

        #[test]
        fn test_table_header_only() {
            // GFM Example 202: Only header row, no data rows
            let input = "| abc | def |\n| --- | --- |\n";
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();

            assert_eq!(tokens.len(), 1);
            if let Token::Table {
                header,
                align,
                rows,
            } = &tokens[0]
            {
                assert_eq!(header.len(), 2);
                assert_eq!(align, &vec![TableAlign::None, TableAlign::None]);
                assert!(rows.is_empty());
            } else {
                panic!("Expected table token");
            }
        }

        #[test]
        fn test_table_inline_formatting() {
            // GFM Example 205: Emphasis and bold in cells
            let input = "| abc | def |\n| --- | --- |\n| *bar* | **baz** |\n";
            let expected = vec![Token::Table {
                header: vec![vec![text("abc")], vec![text("def")]],
                align: vec![TableAlign::None, TableAlign::None],
                rows: vec![vec![
                    vec![Token::Emphasis {
                        text: "bar".into(),
                        tokens: vec![text("bar")],
                    }],
                    vec![Token::Bold {
                        text: "baz".into(),
                        tokens: vec![text("baz")],
                    }],
                ]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_code_spans() {
            // Code spans inside table cells
            let input = "| Command | Description |\n|---------|-------------|\n| `git add` | Stage changes |\n";
            let expected = vec![Token::Table {
                header: vec![vec![text("Command")], vec![text("Description")]],
                align: vec![TableAlign::None, TableAlign::None],
                rows: vec![vec![
                    vec![Token::Code {
                        text: "git add".into(),
                    }],
                    vec![text("Stage changes")],
                ]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_followed_by_blockquote() {
            // GFM Example 206: Table followed by blockquote
            let input = "| abc | def |\n| --- | --- |\n| bar | baz |\n> bar\n";
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();

            assert_eq!(tokens.len(), 2);
            assert!(
                matches!(&tokens[0], Token::Table { .. }),
                "Expected Table token as first token"
            );
            assert!(
                matches!(&tokens[1], Token::BlockQuote { .. }),
                "Expected BlockQuote token as second token"
            );
        }

        #[test]
        fn test_table_bold_link() {
            // Bold text and links in table cells
            let input = "| title | | | | |
|-------|---|---|---|---|
| `[rss]` **[RSS](https://en.wikipedia.org/wiki/RSS) generation from posts** | | | | |
";
            let expected = vec![Token::Table {
                header: vec![vec![text("title")], vec![], vec![], vec![], vec![]],
                align: vec![
                    TableAlign::None,
                    TableAlign::None,
                    TableAlign::None,
                    TableAlign::None,
                    TableAlign::None,
                ],
                rows: vec![vec![
                    vec![
                        Token::Code {
                            text: "[rss]".into(),
                        },
                        text(" "),
                        Token::Bold {
                            text: "[RSS](https://en.wikipedia.org/wiki/RSS) generation from posts"
                                .into(),
                            tokens: vec![
                                Token::Link {
                                    tokens: vec![text("RSS")],
                                    href: "https://en.wikipedia.org/wiki/RSS".into(),
                                    title: None,
                                },
                                text(" generation from posts"),
                            ],
                        },
                    ],
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }

        #[test]
        fn test_table_basic() {
            // Existing table test: table with options
            let input = r"| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `root` | Boolean | `false` | Disable parent inheritance |
";
            let expected = vec![Token::Table {
                header: vec![
                    vec![text("Option")],
                    vec![text("Type")],
                    vec![text("Default")],
                    vec![text("Description")],
                ],
                align: vec![
                    TableAlign::None,
                    TableAlign::None,
                    TableAlign::None,
                    TableAlign::None,
                ],
                rows: vec![vec![
                    vec![Token::Code {
                        text: "root".into(),
                    }],
                    vec![text("Boolean")],
                    vec![Token::Code {
                        text: "false".into(),
                    }],
                    vec![text("Disable parent inheritance")],
                ]],
            }];
            let tokens = parse_lmarkdown(input.as_bytes()).unwrap();
            assert_eq!(tokens, expected);
        }
    }
}
