use std::collections::HashMap;

use log::warn;
use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::{
    lssg_error::LssgError,
    renderer::{InitContext, RenderContext, TokenRenderer, modules::RendererModule},
    sitetree::{Input, Relation, ScriptPlacement, SiteId, SiteNodeKind, SiteTree},
    tree::Dfs,
};
use lmarkdown::Token;
use virtual_dom::{Document, DomNode, DomNodeKind};

/// Id of the layout element that the rendered page is moved into
pub const PAGE_BODY: &str = "pageBody";

/// Tag of the layout element whose children are moved into the document `<head>`
pub const HEAD: &str = "head";

/// All valid layout positions
pub const POSITIONS: [&str; 8] = [
    HEAD,
    "header",
    "beforeBody",
    PAGE_BODY,
    "afterBody",
    "left",
    "right",
    "footer",
];

/// Options for the layout module, configurable per page via TOML attributes.
#[derive(Debug, Clone, Default, Deserialize, Overwrite)]
#[serde(default)]
pub struct LayoutOptions {
    /// Path to a markdown file that describes the layout of the page.
    ///
    /// Relative paths are resolved against the page that configured the layout,
    /// so child pages that inherit the layout resolve the same file.
    ///
    /// ```toml
    /// [layout]
    /// path = "./layout.md"
    /// ```
    ///
    /// **inherited**
    pub path: Option<String>,
}

/// Module that renders a page inside a configurable layout file
///
/// A layout is a markdown file that is configured with the `[layout]` page attribute:
///
/// ```toml
/// [layout]
/// path = "./layout.md"
/// ```
///
/// The layout file contains one HTML element per layout position. The `head` position is a literal `<head>` tag,
/// the other positions (`header`, `beforeBody`, `pageBody`, `afterBody`, `left`, `right`
/// and `footer`) are elements identified by their `id` attribute.
/// All other attributes (like `class` or `style`) are preserved, so custom styling
/// can be applied to each position:
///
/// ```md
/// <head>
/// <link rel="stylesheet" href="layout.css"/>
/// </head>
/// <header id="header" class="my-header">
/// <p>My site</p>
/// </header>
/// <div id="pageBody"></div>
/// <footer id="footer">
/// <p>© My site</p>
/// </footer>
/// ```
///
/// When a page is rendered, its content is moved into the element with `id="pageBody"`
/// (required), the children of the `<head>` element are moved into the document
/// `<head>`, and the remaining elements make up the page frame. Content that already
/// exists inside the `pageBody` element is kept and the page content is appended after it.
///
/// A layout configured on a page is also applied to all of its children. A child can
/// override it with its own `[layout]` table, an empty `[layout]` table disables the
/// layout for the child and its descendants, and `root = true` disables inheritance
/// entirely.
///
/// Resources referenced in the layout file (markdown links, markdown images and
/// `href`/`src` attributes) are discovered and added to the site tree, so they are
/// copied to the output and their paths are translated in every page that uses the
/// layout. Void elements must be self-closed (eg. `<img src="logo.png"/>`).
///
/// This module should be registered after other modules (like `DefaultModule`) so
/// that the layout wraps the fully rendered page.
#[derive(Default)]
pub struct LayoutModule {
    /// Parsed layout file tokens, keyed by the input the layout was read from
    layouts: HashMap<String, Vec<Token>>,
}

/// Resolve the configured layout path to an Input
///
/// Relative paths are resolved against the input of the page that configured the layout,
/// absolute paths and urls are used as-is
fn resolve_layout_input(
    path: &str,
    config_input: &Input,
    client: &reqwest::blocking::Client,
) -> Result<Input, LssgError> {
    let path = path.trim();
    if path.is_empty() {
        return Err(LssgError::parse("layout path is empty"));
    }
    if Input::is_relative(path) {
        config_input.join_single(path, client)
    } else {
        Input::from_string_single(path, client)
    }
}

/// Add all local resources referenced in the layout tokens to the site tree
///
/// Every discovered resource is linked from `page_id` with the same raw path,
/// so its path can be translated to a site tree path at render time
fn discover_layout_resources(
    site_tree: &mut SiteTree,
    page_id: SiteId,
    configured_id: SiteId,
    tokens: &[Token],
    layout_input: &Input,
    client: &reqwest::blocking::Client,
) {
    // add resources next to the page that configured the layout
    let parent_id = site_tree
        .get(configured_id)
        .and_then(|node| node.parent)
        .unwrap_or(site_tree.root());

    let mut stack: Vec<&[Token]> = vec![tokens];
    // TODO: use logic in SiteTree for this
    while let Some(tokens) = stack.pop() {
        for token in tokens {
            let raw_path = match token {
                Token::Autolink { href, .. } if Input::is_local(href) && !href.starts_with('#') => {
                    Some(href)
                }
                Token::Link { href, .. } if Input::is_local(href) && !href.starts_with('#') => {
                    Some(href)
                }
                Token::Image { src, .. } if Input::is_local(src) => Some(src),
                Token::Html { attributes, .. } => attributes
                    .get("href")
                    .or_else(|| attributes.get("src"))
                    .filter(|path| Input::is_local(path) && !path.starts_with('#')),
                _ => None,
            };

            if let Some(raw_path) = raw_path {
                let Ok(inputs) = layout_input.join(raw_path, client).inspect_err(|e| {
                    warn!(
                        "Failed to resolve layout resource '{raw_path}' against '{layout_input}': {e}"
                    )
                }) else {
                    continue;
                };
                for input in inputs {
                    let Ok(resource_id) =
                        site_tree.add_from_input(input, parent_id).inspect_err(|e| {
                            warn!("Failed to add layout resource '{raw_path}' to site tree: {e}")
                        })
                    else {
                        continue;
                    };
                    site_tree.add_link(
                        page_id,
                        resource_id,
                        Relation::Discovered {
                            raw_path: raw_path.clone(),
                        },
                    );
                }
            }

            if let Some(child_tokens) = token.get_tokens() {
                stack.extend(child_tokens.iter().map(|tokens| tokens.as_slice()));
            }
        }
    }
}

/// Rewrite local href/src attributes in the rendered layout to site tree paths
///
/// Only rewrites attributes that were discovered during `init`, so external
/// urls and unresolvable paths are left untouched
fn translate_layout_resources(layout_root: &DomNode, ctx: &RenderContext) {
    let elements: Vec<DomNode> = layout_root.descendants().collect();
    for mut element in elements {
        // clone the matching paths out so the borrow on the element ends
        let paths: Vec<(String, String)> =
            if let DomNodeKind::Element { attributes, .. } = &*element.kind() {
                ["href", "src"]
                    .iter()
                    .filter_map(|key| {
                        attributes
                            .get(*key)
                            .filter(|path| Input::is_local(path) && !path.starts_with('#'))
                            .map(|path| (key.to_string(), path.clone()))
                    })
                    .collect()
            } else {
                Vec::new()
            };
        for (key, raw_path) in paths {
            if let Some(to) = ctx
                .site_tree
                .resources_from_discovered_links(ctx.site_id, &raw_path)
                .into_iter()
                .next()
            {
                element.set_attribute(&key, &ctx.site_tree.path(to));
            }
        }
    }
}

/// Get the site tree paths of the resources that are already emitted into the
/// document `<head>` by other modules (currently the default module), so the same
/// resource is not linked twice
fn head_emitted_paths(site_tree: &SiteTree, site_id: SiteId) -> Vec<String> {
    site_tree
        .links_from(site_id)
        .into_iter()
        .filter(|link| {
            matches!(
                link.relation,
                Relation::External | Relation::Discovered { .. }
            )
        })
        .filter_map(|link| {
            let node = &site_tree[link.to];
            let emitted = match &node.kind {
                SiteNodeKind::Stylesheet(_) => true,
                SiteNodeKind::Resource(_) => {
                    node.name() == "favicon.ico" || node.name().ends_with("js")
                }
                SiteNodeKind::Javascript(javascript) => {
                    javascript.mode().placement() == ScriptPlacement::Head
                }
                _ => false,
            };
            emitted.then(|| site_tree.path(link.to))
        })
        .collect()
}

impl RendererModule for LayoutModule {
    fn id(&self) -> &'static str {
        "layout"
    }

    /// Parse all configured layout files and add the resources they reference to the site tree
    fn init(
        &mut self,
        InitContext {
            http_client: client,
            site_tree,
        }: InitContext,
    ) -> Result<(), LssgError> {
        let pages: Vec<SiteId> = Dfs::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();

        for page_id in pages {
            // Find the page that configures the layout for this page.
            // Extract owned values so the borrow on self/site_tree ends before mutating them.
            let Some((path, config_input, configured_id)) = self
                .find_option::<LayoutOptions>(page_id, site_tree)
                .and_then(|(LayoutOptions { path }, page, configured_id)| {
                    let Some(path) = path else {
                        return None;
                    };
                    let Some(input) = page.input() else {
                        warn!("Page {configured_id} has a layout configured but no input");
                        return None;
                    };
                    Some((path, input.clone(), configured_id))
                })
            else {
                continue;
            };

            let layout_input = match resolve_layout_input(&path, &config_input, client) {
                Ok(input) => input,
                Err(e) => {
                    warn!("Failed to resolve layout path '{path}' for page {page_id}: {e}");
                    continue;
                }
            };

            let key = layout_input.to_string();
            if !self.layouts.contains_key(&key) {
                let Ok(readable) = layout_input.readable() else {
                    warn!("Failed to read layout file '{layout_input}'");
                    continue;
                };
                match lmarkdown::parse_lmarkdown(readable) {
                    Ok(tokens) => {
                        self.layouts.insert(key.clone(), tokens);
                    }
                    Err(e) => {
                        warn!("Failed to parse layout file '{layout_input}': {e}");
                        continue;
                    }
                }
            }
            let Some(tokens) = self.layouts.get(&key) else {
                continue;
            };

            discover_layout_resources(
                site_tree,
                page_id,
                configured_id,
                tokens,
                &layout_input,
                client,
            );
        }

        Ok(())
    }

    fn after_render<'n>(
        &mut self,
        document: &mut Document,
        ctx: &RenderContext<'n>,
        tr: &mut TokenRenderer,
    ) {
        // Find the layout configured for this page (or inherited from an ancestor)
        let Some(layout_input) = self
            .find_option::<LayoutOptions>(ctx.site_id, ctx.site_tree)
            .and_then(|(LayoutOptions { path }, page, _)| {
                let Some(path) = path else {
                    return None;
                };
                let Some(input) = page.input() else {
                    return None;
                };
                Some((path, input.clone()))
            })
            .and_then(|(path, input)| {
                resolve_layout_input(&path, &input, ctx.http_client)
                    .inspect_err(|e| warn!("Failed to resolve layout path '{path}': {e}"))
                    .ok()
            })
        else {
            return;
        };

        let Some(tokens) = self.layouts.get(&layout_input.to_string()) else {
            warn!("Layout file '{layout_input}' was not parsed during init, skipping layout");
            return;
        };

        // render the layout file through the module chain into a detached root
        let layout_root = document.create_element("div");
        tr.render(document, ctx, layout_root.clone(), tokens);

        // the pageBody slot is required
        let Some(page_body) = layout_root.get_element_by_id(PAGE_BODY) else {
            warn!(
                "Layout file '{layout_input}' has no element with id='{PAGE_BODY}', skipping layout"
            );
            return;
        };

        // translate local resource paths in the layout to site tree paths,
        // before any elements are moved out of the layout
        translate_layout_resources(&layout_root, ctx);

        // move the head slot's children into the document head,
        // dropping the head element itself
        if let Some(head) = layout_root
            .get_elements_by_tag_name(HEAD)
            .into_iter()
            .next()
        {
            let emitted = head_emitted_paths(ctx.site_tree, ctx.site_id);
            for child in head.children() {
                // skip link/script elements that other modules already emit into the
                // head, so the same resource is not linked twice
                let skip = if let DomNodeKind::Element {
                    tag, attributes, ..
                } = &*child.kind()
                {
                    let resource = attributes.get("href").or_else(|| attributes.get("src"));
                    match tag.as_str() {
                        "link"
                            if attributes.len() == 2
                                && attributes.get("rel").is_some_and(|rel| rel == "stylesheet") =>
                        {
                            resource.is_some_and(|path| emitted.contains(path))
                        }
                        "script" if attributes.len() == 1 => {
                            resource.is_some_and(|path| emitted.contains(path))
                        }
                        _ => false,
                    }
                } else {
                    false
                };
                if !skip {
                    document.head.append_child(child);
                }
            }
            head.detach();
        }

        // move the rendered page into the pageBody slot
        let body = document.body.clone();
        for child in body.children() {
            page_body.append_child(child);
        }

        // replace the body content with the layout
        for child in layout_root.children() {
            body.append_child(child);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::{Renderer, modules::DefaultModule};
    use std::fs;

    const LAYOUT: &str = r#"<head>
<meta name="layout-test" content="from-layout"/>
</head>
<header id="header" class="custom-header">
<p>Header content</p>
</header>
<div id="pageBody"></div>
<footer id="footer">
<p>Footer content</p>
</footer>"#;

    /// Create a unique temp dir so tests can run in parallel
    fn setup_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("lssg-layout-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Get the site id of the page rendered from the given file name
    fn page_id_for_file(site_tree: &SiteTree, file_name: &str) -> SiteId {
        site_tree
            .pages()
            .find(|(_, page)| {
                page.input()
                    .map(|input| input.to_string().ends_with(file_name))
                    .unwrap_or(false)
            })
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("page '{file_name}' not found in site tree"))
    }

    /// Create a site tree with a root page that configures a layout,
    /// a child page that inherits it and a page with `root = true` that does not
    fn create_layout_site_tree(
        dir: &std::path::Path,
        layout: &str,
    ) -> (SiteTree, reqwest::blocking::Client) {
        fs::write(dir.join("layout.md"), layout).unwrap();
        fs::write(
            dir.join("index.md"),
            r#"<!--
[layout]
path = "layout.md"
-->

# Root Page

[Child](child.md)

[No Layout](no_layout.md)
"#,
        )
        .unwrap();
        fs::write(dir.join("child.md"), "# Child Page\n").unwrap();
        fs::write(
            dir.join("no_layout.md"),
            r#"<!--
root = true
-->

# No Layout Page
"#,
        )
        .unwrap();

        let client = reqwest::blocking::Client::new();
        let input =
            Input::from_string_single(dir.join("index.md").to_str().unwrap(), &client).unwrap();
        let site_tree = SiteTree::from_input(input, client.clone()).unwrap();
        (site_tree, client)
    }

    /// Create a renderer with the default and layout module, like in `create_renderer`
    fn create_renderer(site_tree: &mut SiteTree, client: &reqwest::blocking::Client) -> Renderer {
        let mut renderer = Renderer::default();
        renderer.add_module(DefaultModule::default());
        renderer.add_module(LayoutModule::default());
        renderer.init(site_tree, client);
        renderer.after_init(site_tree);
        renderer
    }

    #[test]
    fn test_layout_rendered_and_inherited() {
        let dir = setup_dir("render");
        let (mut site_tree, client) = create_layout_site_tree(&dir, LAYOUT);
        let mut renderer = create_renderer(&mut site_tree, &client);

        let root_id = page_id_for_file(&site_tree, "index.md");
        let child_id = page_id_for_file(&site_tree, "child.md");
        let no_layout_id = page_id_for_file(&site_tree, "no_layout.md");

        // the root page gets the layout
        let root_html = renderer.render(&site_tree, root_id, &client).unwrap();
        assert!(
            root_html.contains("custom-header"),
            "custom styling should be preserved"
        );
        assert!(root_html.contains("Header content"));
        assert!(root_html.contains("Footer content"));
        assert!(
            root_html.contains(r#"name="layout-test""#),
            "head slot content should be moved into the document head"
        );
        assert_eq!(
            root_html.matches("<head>").count(),
            1,
            "the head element itself should not be rendered into the body"
        );
        // the page content is inside the pageBody slot
        let slot = root_html.find(r#"id="pageBody""#).expect("pageBody slot");
        let content = root_html
            .find("<h1>Root Page</h1>")
            .expect("root page content");
        assert!(
            content > slot,
            "page content should be inside the pageBody slot"
        );

        // the child page inherits the layout
        let child_html = renderer.render(&site_tree, child_id, &client).unwrap();
        assert!(child_html.contains("custom-header"));
        let slot = child_html.find(r#"id="pageBody""#).expect("pageBody slot");
        let content = child_html
            .find("<h1>Child Page</h1>")
            .expect("child page content");
        assert!(
            content > slot,
            "inherited layout should wrap the child page content"
        );

        // a page with root = true does not get the inherited layout
        let no_layout_html = renderer.render(&site_tree, no_layout_id, &client).unwrap();
        assert!(!no_layout_html.contains("custom-header"));
        assert!(!no_layout_html.contains(r#"id="pageBody""#));
        assert!(no_layout_html.contains("<h1>No Layout Page</h1>"));

        let _ = fs::remove_dir_all(&dir);
    }

    // the resource content is only read when writing to the output,
    // so the file does not need to be a valid png
    #[test]
    fn test_layout_resources_discovered() {
        let dir = setup_dir("resources");
        fs::write(dir.join("logo.png"), b"fake png").unwrap();
        let layout = r#"<header id="header">
<img src="logo.png"/>
</header>
<div id="pageBody"></div>"#;
        let (mut site_tree, client) = create_layout_site_tree(&dir, layout);
        let mut renderer = create_renderer(&mut site_tree, &client);

        // the image referenced in the layout is added to the site tree
        let has_logo_resource = site_tree.ids().iter().any(|id| {
            let node = site_tree.get(*id).unwrap();
            matches!(&node.kind, crate::sitetree::SiteNodeKind::Resource(_))
                && node.name() == "logo.png"
        });
        assert!(
            has_logo_resource,
            "layout resources should be added to the site tree"
        );

        // the src is translated to a site tree path in every page that uses the layout
        let root_id = page_id_for_file(&site_tree, "index.md");
        let child_id = page_id_for_file(&site_tree, "child.md");
        let root_html = renderer.render(&site_tree, root_id, &client).unwrap();
        let child_html = renderer.render(&site_tree, child_id, &client).unwrap();
        assert!(root_html.contains(r#"src="/logo.png""#));
        assert!(child_html.contains(r#"src="/logo.png""#));

        let _ = fs::remove_dir_all(&dir);
    }
}
