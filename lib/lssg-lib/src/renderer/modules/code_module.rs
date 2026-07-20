use std::{cell::OnceCell, collections::HashMap};

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

const COPY_BUTTON_CSS: &str = include_str!("./code_module/copy_button.css");
const COPY_BUTTON_JS: &str = include_str!("./code_module/copy_button.js");

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

    /// Show a button on code blocks that copies their content to the clipboard.
    ///
    /// ```toml
    /// code.copy_button = false
    /// ```
    ///
    /// This is a site-wide setting, since a single highlight.js script is shared
    /// across all pages, so it can only be configured on the root page.
    ///
    /// default: `true`
    pub copy_button: bool,

    /// Localize copy button text using the page's language setting.
    ///
    /// When disabled, button text defaults to English.
    ///
    /// ```toml
    /// code.copy_button_language = false
    /// ```
    ///
    /// default: `true`
    pub copy_button_language: Option<String>,

    /// Hide the copy button by default and show it on hover/focus.
    ///
    /// When disabled, the button is always visible.
    ///
    /// ```toml
    /// code.copy_button_autohide = false
    /// ```
    ///
    /// default: `false`
    pub copy_button_autohide: bool,
}

impl Default for CodeModuleOptions {
    fn default() -> Self {
        Self {
            theme_path: None,
            auto_detect_language: true,
            copy_button: true,
            copy_button_language: None,
            copy_button_autohide: false,
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

        let js_id = site_tree.add(SiteNode::javascript(
            "highlight.min.js",
            site_tree.root(),
            Javascript::from(HIGHLIGHTJS_JS).with_mode(ScriptMode::Blocking),
        ));
        let copy_button_js_id: OnceCell<SiteId> = OnceCell::new();
        let copy_button_css_id: OnceCell<SiteId> = OnceCell::new();
        // Once cell take lazy load the default theme
        let css_id: OnceCell<SiteId> = OnceCell::new();
        // map of all themed css ids
        let mut themes_css_site_ids: HashMap<String, SiteId> = HashMap::new();
        let mut run_js_site_ids: HashMap<String, SiteId> = HashMap::new();

        // Link resources only to pages that have code blocks
        for site_id in code_pages {
            // get theme path from first found page with this option, use that page to parse relative theme path
            #[derive(serde::Deserialize)]
            struct ThemePath {
                theme_path: String,
            }
            let theme_css_site_id: SiteId = self
                .find_option::<ThemePath>(site_id, site_tree)
                .and_then(|(ThemePath { theme_path }, page, id)| {
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
                    Some(
                        *themes_css_site_ids
                            .entry(input.to_string())
                            .or_insert_with(|| {
                                site_tree.add(SiteNode::stylesheet(filename, id, stylesheet))
                            }),
                    )
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
            site_tree.add_link(site_id, theme_css_site_id, Relation::External);
            site_tree.add_link(site_id, js_id, Relation::External);

            let opts: CodeModuleOptions = self.propegated_options(site_id, site_tree);
            if opts.copy_button {
                let js_id = *copy_button_js_id.get_or_init(|| {
                    site_tree.add(SiteNode::javascript(
                        "highlight-copy-button.min.js",
                        site_tree.root(),
                        Javascript::from(COPY_BUTTON_JS).with_mode(ScriptMode::Blocking),
                    ))
                });
                let css_id = *copy_button_css_id.get_or_init(|| {
                    site_tree.add(SiteNode::stylesheet(
                        "highlight-copy-button.min.css",
                        site_tree.root(),
                        Stylesheet::from(COPY_BUTTON_CSS),
                    ))
                });
                site_tree.add_link(site_id, js_id, Relation::External);
                site_tree.add_link(site_id, css_id, Relation::External);
            }

            let mut run_id = format!("auto:{}", opts.auto_detect_language);
            if opts.copy_button {
                run_id.push_str(&format!(
                    "ah:{}lang:{:?}",
                    opts.copy_button_autohide, opts.copy_button_language
                ));
            }
            let run_site_id = *run_js_site_ids.entry(run_id.clone())
              .or_insert_with(|| {
                let mut js = String::new();
                if opts.copy_button {
                  js.push_str("hljs.addPlugin(new CopyButtonPlugin({");
                  js.push_str(&format!("autohide: {},", opts.copy_button_autohide));
                  if let Some(lang) = opts.copy_button_language {
                    js.push_str(&format!("lang: {lang},"));
                  }

                  js.push_str("}));");
                }
                if opts.auto_detect_language {
                  js.push_str("hljs.highlightAll();");
                } else {
                  js.push_str("document.querySelectorAll('pre code').forEach(el => {");
                  js.push_str("  if (!el.matches('[class*=\"language-\"]')) el.classList.add('language-plaintext');");
                  js.push_str("  hljs.highlightElement(el);");
                  js.push_str("});");
                }
                site_tree.add(SiteNode::javascript(
                    &format!("highlight-run-{run_id}.js"),
                    site_tree.root(),
                    Javascript::from(js).with_mode(ScriptMode::Blocking),
                ))
            });
            site_tree.add_link(site_id, run_site_id, Relation::External);
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
