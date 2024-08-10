use crate::{
    lmarkdown::{parse_lmarkdown, Token},
    lssg_error::LssgError,
};

use super::Input;

/// A SiteTree node representing a page made by a markdown file
#[derive(Debug)]
pub struct Page {
    tokens: Vec<Token>,
}
impl Page {
    pub fn empty() -> Page {
        Page { tokens: vec![] }
    }

    pub fn from_input(input: &Input) -> Result<Page, LssgError> {
        let tokens = parse_lmarkdown(input.readable()?)?;
        Ok(Page { tokens })
    }

    /// Discover any links inside of the page will return vec with (text, href)
    pub fn links(&self) -> Vec<(&Vec<Token>, &String, &Option<String>)> {
        let mut hrefs = vec![];
        let mut queue: Vec<Vec<&Token>> = vec![self.tokens.iter().collect()];
        while let Some(tokens) = queue.pop() {
            for t in tokens {
                if let Token::Link {
                    tokens: text,
                    href,
                    title,
                } = t
                {
                    hrefs.push((text, href, title));
                    continue;
                }
                if let Some(tokens) = t.get_tokens() {
                    queue.push(tokens);
                }
            }
        }
        return hrefs;
    }

    /// Discover any images inside of the page
    pub fn images(&self) -> Vec<(&Vec<Token>, &String, &Option<String>)> {
        let mut srcs = vec![];
        let mut queue: Vec<Vec<&Token>> = vec![self.tokens.iter().collect()];
        while let Some(tokens) = queue.pop() {
            for t in tokens {
                if let Token::Image { tokens, src, title } = t {
                    srcs.push((tokens, src, title));
                    continue;
                }
                if let Some(tokens) = t.get_tokens() {
                    queue.push(tokens);
                }
            }
        }
        return srcs;
    }

    pub fn attributes(&self) -> Option<&toml::Table> {
        if let Some(Token::Attributes { table }) = self.tokens().first() {
            Some(table)
        } else {
            None
        }
    }

    pub fn attributes_mut(&mut self) -> Option<&mut toml::Table> {
        if let Some(Token::Attributes { table }) = self.tokens_mut().first_mut() {
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
    pub fn tokens_mut(&mut self) -> &mut Vec<Token> {
        &mut self.tokens
    }
}
