use crate::{
    lmarkdown::{Token, parse_lmarkdown},
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
        let tokens = parse_lmarkdown(input.readable()?).map_err(|e| {
            LssgError::from(e).with_context(format!("Failed to parse markdown {input:?}"))
        })?;
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
        hrefs
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
        srcs
    }

    pub fn attributes(&self) -> Option<toml::Table> {
        if let Some(Token::Attributes { table }) = self.tokens().first() {
            Some(table.clone())
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
        href.ends_with(".md") && Input::is_relative(href)
    }

    /// Return the list of top tokens in the page
    /// NOTE: a token can contain more tokens
    pub fn tokens(&self) -> &Vec<Token> {
        &self.tokens
    }

    pub fn tokens_mut(&mut self) -> &mut Vec<Token> {
        &mut self.tokens
    }

    /// Iterate over all tokens recursively, including child tokens in links, images, etc.
    /// Returns an iterator that yields references to all tokens in depth-first order.
    pub fn iter_all_tokens(&self) -> impl Iterator<Item = &Token> {
        AllTokensIter {
            queue: self.tokens.iter().collect(),
        }
    }
}

struct AllTokensIter<'a> {
    queue: Vec<&'a Token>,
}

impl<'a> Iterator for AllTokensIter<'a> {
    type Item = &'a Token;

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.queue.pop()?;

        // If this token has child tokens, add them to the queue
        if let Some(child_tokens) = token.get_tokens() {
            // Reverse the order so they're processed in the correct order
            self.queue.extend(child_tokens.into_iter().rev());
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
        let page = Page { tokens };

        // Collect all tokens
        let all_tokens: Vec<&Token> = page.iter_all_tokens().collect();

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
        let page = Page { tokens };

        let all_tokens: Vec<&Token> = page.iter_all_tokens().collect();

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
