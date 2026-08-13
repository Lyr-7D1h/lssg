use std::io::Read;

use log::warn;

use super::RenderContext;
use crate::renderer::modules::{DefaultModule, RendererModule};
use lmarkdown::Token;
use virtual_dom::{Document, DomNode};

/// Used for recursively rendering
pub struct TokenRenderer {
    modules: *mut Vec<Box<dyn RendererModule>>,
}

impl<'a> TokenRenderer {
    /// Parse `input` into a dom node using a simplified rendered using just the default module
    pub fn parse_lmarkdown(
        input: impl Read,
        ctx: &RenderContext,
    ) -> lmarkdown::Result<Vec<DomNode>> {
        let tokens = lmarkdown::parse_lmarkdown(input)?;
        let mut document = Document::new();
        let mut default = vec![Box::new(DefaultModule::default()) as Box<dyn RendererModule>];
        let mut tr = TokenRenderer::new(&mut default);
        let body = document.body.clone();
        tr.render(&mut document, ctx, body, &tokens);
        Ok(document.body.children().collect())
    }

    pub fn new(modules: &'a mut Vec<Box<dyn RendererModule>>) -> TokenRenderer {
        // turn into pointer to allow for recursive call backs in render()
        let modules: *mut Vec<Box<dyn RendererModule>> = modules;
        TokenRenderer { modules }
    }

    /// Render using other modules down the rendering chain
    pub fn render_down(
        &mut self,
        current_module: &dyn RendererModule,
        document: &mut Document,
        ctx: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &[Token],
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            let mut modules_iter = modules.iter_mut();
            // skip all before current module
            for module in modules_iter.by_ref() {
                if module.id() == current_module.id() {
                    break;
                }
            }
            for module in modules_iter {
                if current_module.id() == module.id() {
                    continue;
                }
                if let Some(p) = module.render_token(document, ctx, parent.clone(), token, self) {
                    parent = p;
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
        parent
    }

    pub fn render(
        &mut self,
        document: &mut Document,
        ctx: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &[Token],
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if let Some(p) = module.render_token(document, ctx, parent.clone(), token, self) {
                    parent = p;
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
        parent
    }

    /// consume self and return a parsed domtree
    pub fn start_render(mut self, document: &mut Document, ctx: &RenderContext) {
        let tokens = ctx.page.tokens();
        self.render(document, ctx, document.body.clone(), tokens);
    }
}

/// Count meaningful tokens that should produce output (Attributes, Comment, and empty Link are skipped)
fn count_meaningful_tokens(tokens: &[lmarkdown::Token]) -> usize {
    tokens
        .iter()
        .filter(|t| {
            if let lmarkdown::Token::Link { tokens, .. } = t {
                !tokens.is_empty()
            } else {
                !matches!(
                    t,
                    lmarkdown::Token::Attributes { .. } | lmarkdown::Token::Comment { .. }
                )
            }
        })
        .count()
}

/// Count top-level children of a DomNode
fn count_top_level_children(node: &virtual_dom::DomNode) -> usize {
    node.children().count()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Cursor;

    use super::*;
    use crate::renderer::RenderContext;
    use crate::sitetree::{Input, SiteId, SiteTree};

    fn create_test_render_ctx<'a>() -> Box<RenderContext<'a>> {
        let http_client = reqwest::blocking::Client::new();

        // Create a minimal temporary page
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("lssg_test_page.md");
        fs::write(&test_file, "").unwrap();

        let input = Input::from_string_single(&test_file.to_str().unwrap(), &http_client).unwrap();

        let tree = SiteTree::from_input(input.clone(), &http_client).unwrap();
        let page = tree.page(SiteId(0)).unwrap();

        Box::new(RenderContext {
            http_client: &http_client,
            site_tree: &tree,
            site_id: SiteId(0),
            page,
            input: Some(&input),
        })
    }

    #[test]
    fn test_parse_lmarkdown_one_to_one_token_to_html() {
        let ctx = create_test_render_ctx();

        // Test various markdown inputs to verify 1:1 token-to-HTML mapping
        let test_cases = vec![
            ("# Heading", "single heading"),
            ("Paragraph text", "single paragraph"),
            ("# H\nP\n\n**bold**", "heading + paragraph + bold"),
            ("- item 1\n- item 2", "bullet list with two items"),
            ("> quote", "blockquote"),
            ("`code`", "inline code"),
            ("`code block`", "code block"),
            ("---", "thematic break"),
            ("text\nmore text", "text with line break"),
            ("[link](http://example.com)", "link"),
        ];

        for (markdown, description) in test_cases {
            let tokens = lmarkdown::parse_lmarkdown(Cursor::new(markdown)).unwrap();
            let meaningful = count_meaningful_tokens(&tokens);

            let result = TokenRenderer::parse_lmarkdown(Cursor::new(markdown), &ctx).unwrap();

            let rendered = count_top_level_children(&result);

            assert_eq!(
                rendered,
                meaningful,
                "Token-to-HTML mapping mismatch: description={}, input='{}', tokens={}, meaningful={}, rendered={}",
                description,
                markdown,
                tokens.len(),
                meaningful,
                rendered
            );
        }
    }
}
