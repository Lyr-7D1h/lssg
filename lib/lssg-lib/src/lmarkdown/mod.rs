use std::{fs::File, io::Read, path::Path};

use crate::lssg_error::LssgError;

use self::{char_reader::CharReader, parse_error::ParseError};

pub mod char_reader;
mod lexer;
pub use lexer::*;
pub mod parse_error;

pub fn parse_lmarkdown(input: impl Read) -> Result<Vec<Token>, ParseError> {
    let mut lexer = LMarkdownLexer::new(CharReader::new(input));

    let mut tokens = vec![];

    loop {
        match lexer.read_token()? {
            Token::EOF => break,
            t => tokens.push(t),
        }
    }

    Ok(tokens)
}
pub fn parse_lmarkdown_from_file(path: &Path) -> Result<Vec<Token>, LssgError> {
    let file = File::open(path)
        .map_err(|e| LssgError::from(e).with_context(format!("could not open {path:?}")))?;

    return Ok(parse_lmarkdown(file)?);
}

mod tests {
    use std::{collections::HashMap, io::Cursor};

    use crate::domtree::to_attributes;

    use super::*;

    #[test]
    fn test_html() {
        let input = r#"<i class="fa-solid fa-rss"></i><button disabled></button>"#;
        let expected = vec![
            Token::Html {
                tag: "i".into(),
                attributes: to_attributes([("class", "fa-solid fa-rss")]),
                tokens: vec![],
            },
            Token::Html {
                tag: "button".into(),
                attributes: to_attributes([("disabled", "")]),
                tokens: vec![],
            },
        ];

        let reader: Box<dyn Read> = Box::new(Cursor::new(input));
        let tokens = parse_lmarkdown(reader).unwrap();
        assert_eq!(tokens, expected);
    }
}
