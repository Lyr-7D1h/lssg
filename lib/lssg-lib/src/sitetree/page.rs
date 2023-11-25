use crate::{
    lmarkdown::{parse_lmarkdown, Token},
    lssg_error::LssgError,
};

use super::Input;

#[derive(Debug)]
pub struct Page {
    tokens: Vec<Token>,
}
impl Page {
    pub fn from_input(input: &Input) -> Result<Page, LssgError> {
        let tokens = parse_lmarkdown(input.readable()?)?;
        Ok(Page { tokens })
    }

    /// Discover any resources and links inside of the page will return vec with (text, href)
    pub fn links(&self) -> Vec<(&Vec<Token>, &String)> {
        let mut hrefs = vec![];
        let mut queue = vec![&self.tokens];
        while let Some(tokens) = queue.pop() {
            for t in tokens {
                match t {
                    Token::Heading { tokens, .. } => queue.push(tokens),
                    Token::Paragraph { tokens, .. } => queue.push(tokens),
                    Token::Html { tokens, .. } => queue.push(tokens),
                    Token::Link { href, text } => {
                        hrefs.push((text, href));
                    }
                    _ => {}
                }
            }
        }
        return hrefs;
    }

    pub fn attributes(&self) -> Option<&toml::Table> {
        if let Some(Token::Attributes { table }) = self.tokens().first() {
            Some(table)
        } else {
            None
        }
    }

    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }
}
