use std::cell::OnceCell;

use log::{error, info, warn};
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::{InitContext, RenderContext, TokenRenderer, modules::RendererModule},
    sitetree::{Javascript, Relation, ScriptMode, SiteId, SiteNode, SiteTree, Stylesheet},
};
use virtual_dom::{Document, DomNode};

// https://highlightjs.org/download
const HIGHLIGHTJS_CSS: &str = include_str!("./code_module/github.min.css");
const HIGHLIGHTJS_JS: &str = include_str!("./code_module/highlight.min.js");

/// Options for the code module, configurable per page via TOML attributes.
#[derive(Debug, Clone, serde_extensions::Overwrite)]
pub struct CodeModuleOptions {
    /// Custom theme for code blocks. By default the *Github* theme is used.
    ///
    /// Download styles from [github](https://github.com/highlightjs/highlight.js/tree/main/src/styles) or view the [demo](https://highlightjs.org/examples) first.
    ///
    /// Once downloaded specify the path to this theme relative to this file.
    ///
    /// ```toml
    /// code.theme_path = "./github.css"
    /// ```
    ///
    /// **inherited**
    pub theme_path: Option<String>,

    /// Automatically detect the language of code blocks that don't specify one
    /// (e.g. a fence without an info string like ` ``` `).
    ///
    /// When disabled, only code blocks with an explicit language (e.g. ` ```rust `)
    /// are syntax highlighted; others are left as plain text.
    ///
    /// This is a site-wide setting, since a single highlight.js script is shared
    /// across all pages, so it can only be configured on the root page.
    ///
    /// ```toml
    /// code.auto_detect_language = false
    /// ```
    ///
    /// default: `true`
    pub auto_detect_language: bool,
}

impl Default for CodeModuleOptions {
    fn default() -> Self {
        Self {
            theme_path: None,
            auto_detect_language: true,
        }
    }
}

/// Finds all page IDs that contain at least one code block
fn find_code_block_pages(site_tree: &SiteTree) -> Vec<SiteId> {
    site_tree
        .pages()
        .filter(|(_, page)| {
            page.tokens()
                .iter()
                .any(|t| matches!(t, Token::CodeBlock { .. }))
        })
        .map(|(id, _)| id)
        .collect()
}

/// Module that handles code blocks with highlight.js syntax highlighting.
///
/// Detects pages that contain code blocks and only links the
/// highlight.js CSS and JS resources to those pages.
#[derive(Default)]
pub struct CodeModule {}

impl RendererModule for CodeModule {
    fn id(&self) -> &'static str {
        "code"
    }

    /// Add highlight.js resources only to pages that have code blocks,
    /// plus any custom CSS defined via the `code` page attribute.
    fn init(
        &mut self,
        InitContext {
            site_tree,
            http_client,
            ..
        }: InitContext,
    ) -> Result<(), LssgError> {
        let code_pages = find_code_block_pages(site_tree);

        if code_pages.is_empty() {
            return Ok(());
        }

        info!(
            "Found {} page(s) with code blocks, adding highlight.js resources",
            code_pages.len()
        );

        // This is a site-wide setting (the highlight.js script is shared across
        // all pages), so it's only read from the root page's own attributes.
        let options: CodeModuleOptions = site_tree
            .page(site_tree.root())
            .and_then(|page| self.options(page))
            .unwrap_or_default();

        let mut script = HIGHLIGHTJS_JS.to_owned();
        if !options.auto_detect_language {
            script.push_str("\nhljs.configure({languages: []});");
        }
        script.push_str("\nhljs.highlightAll();");

        // Add highlight.js resources to the site tree root
        let js_id = site_tree.add(SiteNode::javascript(
            "highlight.min.js",
            site_tree.root(),
            Javascript::from(script).with_mode(ScriptMode::Blocking),
        ));
        // Once cell take lazy load the default theme
        let css_id: OnceCell<SiteId> = OnceCell::new();

        // Link resources only to pages that have code blocks
        for site_id in code_pages {
            let ancestors: Vec<_> = std::iter::once(site_id)
                .chain(site_tree.parents(site_id))
                .collect();

            // go up the tree from current page and find the page with theme_path set
            // If found add theme path to site tree
            let css_id = ancestors
                .into_iter()
                .filter_map(|id| site_tree.page(id).zip(Some(id)))
                .take_while(|(page, _)| !page.has_root_attribute())
                .find_map(|(page, id)| {
                    let Some(theme_path) = self
                        .options::<CodeModuleOptions>(page)
                        .and_then(|o| o.theme_path)
                    else {
                        return None;
                    };
                    let Some(input) = page.input() else {
                        warn!("Page without an originating input has a theme_path set");
                        return None;
                    };
                    Some((theme_path, input.clone(), id))
                })
                .and_then(|(theme_path, input, id)| {
                    let stylesheet = input
                        .join_single(&theme_path, http_client)
                        .inspect_err(|e| error!("Failed to parse theme path '{theme_path}': {e}"))
                        .ok()?
                        .try_into()
                        .inspect_err(|e| {
                            error!("Failed to turn into stylesheet '{theme_path}': {e}")
                        })
                        .ok()?;
                    let filename = input
                        .filename()
                        .inspect_err(|e| error!("Failed to get filename from '{input}': {e}"))
                        .ok()?;
                    Some(site_tree.add(SiteNode::stylesheet(filename, id, stylesheet)))
                })
                .unwrap_or_else(|| {
                    *css_id.get_or_init(|| {
                        site_tree.add(SiteNode::stylesheet(
                            "highlight.min.css",
                            site_tree.root(),
                            Stylesheet::from(HIGHLIGHTJS_CSS),
                        ))
                    })
                });

            site_tree.add_link(site_id, css_id, Relation::External);
            site_tree.add_link(site_id, js_id, Relation::External);
        }

        Ok(())
    }

    fn render_token<'n>(
        &mut self,
        document: &mut Document,
        _ctx: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        _tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        match token {
            Token::CodeBlock { text: code, info } => {
                let mut code_html = document.create_element("code");
                if let Some(info) = info {
                    code_html.set_attribute("class", &format!("language-{info}"));
                }
                code_html.append_child(document.create_text_node(code));
                let pre = document.create_element("pre");
                pre.append_child(code_html);
                parent.append_child(pre);
                Some(parent)
            }
            _ => None,
        }
    }
}
