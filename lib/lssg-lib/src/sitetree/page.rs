use std::collections::{HashMap, hash_map::Iter};

use log::warn;

use crate::{
    lmarkdown::{Token, parse_lmarkdown},
    lssg_error::LssgError,
};

use super::Input;

/// A SiteTree node representing a page made by a markdown file
#[derive(Debug)]
pub struct Page {
    tokens: Vec<Token>,
    input: Option<Input>,
    /// Map a link href or image src to an input
    raw_path_map: HashMap<String, Vec<Input>>,
}
impl Page {
    pub fn empty() -> Page {
        Page {
            tokens: vec![],
            input: None,
            raw_path_map: HashMap::new(),
        }
    }

    pub fn from_input(
        input: Input,
        http_client: &reqwest::blocking::Client,
    ) -> Result<Page, LssgError> {
        let tokens = parse_lmarkdown(input.readable()?).map_err(|e| {
            LssgError::from(e).with_context(format!("Failed to parse markdown {input:?}"))
        })?;
        let mut resource_map = HashMap::new();
        let iter = TokenIterator {
            queue: tokens.iter().collect(),
        };
        for t in iter {
            match t {
                Token::Autolink { href, .. } | Token::Link { href, .. }
                    if Input::is_local(href) =>
                {
                    let Ok(inputs) = input
                        .join(href, http_client)
                        .inspect_err(|e| warn!("Failed to get {input}: {e}"))
                    else {
                        continue;
                    };
                    if inputs.is_empty() {
                        continue;
                    }
                    resource_map.insert(href.to_string(), inputs);
                }
                Token::Image { src, .. } => {
                    let Ok(inputs) = input
                        .join(src, http_client)
                        .inspect_err(|e| warn!("Failed to get {input}: {e}"))
                    else {
                        continue;
                    };
                    if inputs.is_empty() {
                        continue;
                    }
                    resource_map.insert(src.to_string(), inputs);
                }
                _ => {}
            }
        }
        Ok(Page {
            tokens,
            input: Some(input),
            raw_path_map: resource_map,
        })
    }

    pub fn input(&self) -> Option<&Input> {
        self.input.as_ref()
    }

    pub fn raw_path_inputs(&self) -> Iter<'_, String, Vec<Input>> {
        self.raw_path_map.iter()
    }

    pub fn attributes(&self) -> Option<toml::Table> {
        if let Some(Token::Attributes { table }) = self.tokens().first() {
            Some(table.clone())
        } else {
            None
        }
    }

    /// Return the list of top tokens in the page
    /// NOTE: a token can contain more tokens use `iter()` to iterate over all tokens
    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }

    #[cfg(test)]
    pub fn tokens_mut(&mut self) -> &mut Vec<Token> {
        &mut self.tokens
    }

    /// Iterate over all tokens recursively, including child tokens in links, images, etc.
    /// Returns an iterator that yields references to all tokens in depth-first order.
    pub fn tokens_iter(&self) -> impl Iterator<Item = &Token> {
        TokenIterator {
            queue: self.tokens.iter().collect(),
        }
    }
}

struct TokenIterator<'a> {
    queue: Vec<&'a Token>,
}

impl<'a> Iterator for TokenIterator<'a> {
    type Item = &'a Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.queue.pop()?;

        // If this token has child tokens, add them to the queue
        if let Some(child_tokens) = token.get_tokens() {
            // Reverse the order so they're processed in the correct order
            self.queue.extend(child_tokens.into_iter().flatten().rev());
        }

        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iter_all_tokens() {
        // Create a page with nested tokens
        let markdown = r#"# Heading
This is a paragraph with a [link text](http://example.com) inside.
Another paragraph with **bold** and _emphasis_."#;

        let tokens = parse_lmarkdown(markdown.as_bytes()).unwrap();
        let page = Page {
            tokens,
            input: None,
            raw_path_map: HashMap::new(),
        };

        // Collect all tokens
        let all_tokens: Vec<&Token> = page.tokens_iter().collect();

        // We should have more tokens than just the top-level ones
        assert!(all_tokens.len() > page.tokens().len());

        // Check that we can find nested tokens (like the link's text tokens)
        let has_link = all_tokens.iter().any(|t| matches!(t, Token::Link { .. }));
        assert!(has_link, "Should find Link token");

        // Check that we can find tokens inside the link
        let has_text_in_link = all_tokens
            .iter()
            .any(|t| matches!(t, Token::Text { text } if text == "link text"));
        assert!(has_text_in_link, "Should find Text token inside Link");
    }

    #[test]
    fn test_iter_all_tokens_with_lists() {
        let markdown = r#"- Item 1 with [link](url)
- Item 2"#;

        let tokens = parse_lmarkdown(markdown.as_bytes()).unwrap();
        let page = Page {
            tokens,
            input: None,
            raw_path_map: HashMap::new(),
        };

        let all_tokens: Vec<&Token> = page.tokens_iter().collect();

        // Should find the list token
        let has_list = all_tokens
            .iter()
            .any(|t| matches!(t, Token::BulletList { .. }));
        assert!(has_list, "Should find BulletList token");

        // Should find tokens inside list items
        let has_link = all_tokens.iter().any(|t| matches!(t, Token::Link { .. }));
        assert!(has_link, "Should find Link token inside list");
    }
}
