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
                if let Token::Link { tokens: text, href } = t {
                    hrefs.push((text, href));
                    continue;
                }
                if let Some(tokens) = t.get_tokens() {
                    queue.push(tokens);
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

    // only support relative links to markdown files for now
    // because this will allow absolute links to markdown files links to for
    // example https://github.com/Lyr-7D1h/airap/blob/master/README.md
    // will render a readme even though this might not be appropiate
    pub fn is_href_to_page(href: &str) -> bool {
        href.ends_with(".md") && Input::is_relative(&href)
    }

    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }
}
