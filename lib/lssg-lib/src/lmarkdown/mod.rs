use std::{fs::File, io::Read, path::Path};

use crate::lssg_error::LssgError;

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

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

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
                text: "Rust > c++".into(),
                tokens: vec![],
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
}
