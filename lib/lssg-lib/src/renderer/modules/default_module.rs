use std::collections::{HashMap, HashSet};

use log::warn;

use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    lssg_error::LssgError,
    sitetree::{Page, Relation, SiteNode, SiteNodeKind, SiteTree, Stylesheet},
    tree::{Node, DFS},
};
use proc_html::html;
use virtual_dom::{self, to_attributes, Document, DomNode, DomNodeKind};

use crate::renderer::{RenderContext, RendererModule, TokenRenderer};


mod render_html;

const DEFAULT_STYLESHEET: &[u8] = include_bytes!("./default_stylesheet.css");

#[derive(Debug, Clone, Overwrite)]
struct PropegatedOptions {
    /// Add extra resources
    pub title: String,
    /// Translates to meta tags <https://www.w3schools.com/tags/tag_meta.asp>
    pub meta: HashMap<String, String>,
    /// Lang attribute ("en") <https://www.w3schools.com/tags/ref_language_codes.asp>
    pub language: String,
}
impl Default for PropegatedOptions {
    fn default() -> Self {
        Self {
            meta: HashMap::new(),
            title: String::new(),
            language: "en".into(),
        }
    }
}

#[derive(Debug, Clone, Overwrite)]
pub struct SinglePageOptions {
    /// If this page is a root don't reuse options from parent
    pub root: bool,
}
impl Default for SinglePageOptions {
    fn default() -> Self {
        Self { root: false }
    }
}

fn create_options_map(
    module: &DefaultModule,
    site_tree: &SiteTree,
) -> Result<HashMap<usize, PropegatedOptions>, LssgError> {
    let mut options_map: HashMap<usize, PropegatedOptions> = HashMap::new();
    for id in DFS::new(site_tree) {
        if let SiteNodeKind::Page(page) = &site_tree[id].kind {
            if let Some(parent) = site_tree.page_parent(id) {
                if let Some(parent_options) = options_map.get(&parent) {
                    let parent_options = parent_options.clone();
                    let options: PropegatedOptions =
                        module.options_with_default(page, parent_options);
                    options_map.insert(id, options.clone());
                    continue;
                }
            }

            let options: PropegatedOptions = module.options(page);
            options_map.insert(id, options.clone());
        }
    }
    Ok(options_map)
}

/// Implements all basic default behavior, like rendering all tokens and adding meta tags and title to head
pub struct DefaultModule {
    /// Map of all site pages to options. Considers options from parents.
    options_map: HashMap<usize, PropegatedOptions>,
}

impl DefaultModule {
    pub fn new() -> Self {
        Self {
            options_map: HashMap::new(),
        }
    }
}

impl RendererModule for DefaultModule {
    fn id(&self) -> &'static str {
        "default"
    }

    /// Add all resources from ResourceOptions to SiteTree
    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError> {
        let mut relation_map = HashMap::new();

        let pages: Vec<usize> = DFS::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();

        let default_stylesheet = site_tree.add(SiteNode::stylesheet(
            "default.css",
            site_tree.root(),
            Stylesheet::from_readable(DEFAULT_STYLESHEET)?,
        ))?;

        // propegate relations to stylesheets and favicon from parent to child
        for id in pages {
            // add default stylesheet to all pages
            site_tree.add_link(id, default_stylesheet);

            // skip page if disabled
            if let SiteNodeKind::Page(page) = &site_tree[id].kind {
                let opts: SinglePageOptions = self.options(page);
                if opts.root {
                    continue;
                }
            }

            // get the set of links to favicon and stylesheets
            let mut set: HashSet<usize> = site_tree
                .links_from(id)
                .into_iter()
                .filter_map(|link| match link.relation {
                    Relation::External | Relation::Discovered { .. } => {
                        let node = &site_tree[link.to];
                        match node.kind {
                            SiteNodeKind::Stylesheet { .. } => Some(link.to),
                            SiteNodeKind::Resource { .. } if node.name == "favicon.ico" => {
                                Some(link.to)
                            }
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .collect();

            // update set with parent and add any links from parent
            if let Some(parent) = site_tree.page_parent(id) {
                if let Some(parent_set) = relation_map.get(&parent) {
                    // add links from parent_set without the ones it already has
                    for to in (parent_set - &set).iter() {
                        site_tree.add_link(id, *to);
                    }
                    set = set.union(parent_set).cloned().collect();
                }
            }
            relation_map.insert(id, set);
        }

        Ok(())
    }

    fn after_init(&mut self, site_tree: &SiteTree) -> Result<(), LssgError> {
        // save options map after site tree has been created to get all pages
        self.options_map = create_options_map(&self, site_tree)?;
        Ok(())
    }

    fn render_page<'n>(&mut self, document: &mut Document, context: &RenderContext<'n>) {
        let site_id = context.site_id;
        let site_tree = context.site_tree;

        let options = self
            .options_map
            .get(&site_id)
            .expect("expected options map to contain all page ids");

        // Add language to html tag
        if let DomNodeKind::Element { attributes, .. } = &mut *document.root().kind_mut() {
            attributes.insert("lang".to_owned(), options.language.clone());
        }

        // fill head
        let head = &document.head;

        let title = document.create_element("title");
        title.append_child(document.create_text_node(options.title.clone()));
        head.append_child(title);

        for link in site_tree.links_from(site_id) {
            match link.relation {
                Relation::External | Relation::Discovered { .. } => match site_tree[link.to].kind {
                    SiteNodeKind::Resource { .. } if site_tree[link.to].name == "favicon.ico" => {
                        head.append_child(document.create_element_with_attributes(
                            "link",
                            to_attributes([
                                ("rel", "icon"),
                                ("type", "image/x-icon"),
                                ("href", &site_tree.path(link.to)),
                            ]),
                        ));
                    }
                    SiteNodeKind::Stylesheet { .. } => {
                        head.append_child(document.create_element_with_attributes(
                            "link",
                            to_attributes([
                                ("rel", "stylesheet"),
                                ("href", &site_tree.path(link.to)),
                            ]),
                        ));
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        head.append_child(document.create_element_with_attributes(
            "meta",
            to_attributes([
                ("name", "viewport"),
                ("content", r#"width=device-width, initial-scale=1"#),
            ]),
        ));
        head.append_child(
            document.create_element_with_attributes("meta", to_attributes([("charset", "utf-8")])),
        );
        for (key, value) in &options.meta {
            head.append_child(document.create_element_with_attributes(
                "meta",
                to_attributes([("name", key), ("content", value)]),
            ));
        }

        // add breacrumbs if not root
        if context.site_id != context.site_tree.root() {
            let nav = document
                .create_element_with_attributes("nav", to_attributes([("class", "breadcrumbs")]));

            nav.append_child(document.create_text_node("/"));

            let parents = site_tree.parents(site_id);
            let parents_length = parents.len();
            for (i, p) in parents.into_iter().rev().enumerate() {
                let a = document.create_element_with_attributes(
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                );
                a.append_child(document.create_text_node(site_tree[p].name.clone()));
                nav.append_child(a);
                if i != parents_length - 1 {
                    nav.append_child(document.create_text_node("/"));
                }
            }
            nav.append_child(document.create_text_node(format!("/{}", site_tree[site_id].name)));

            document.body.append_child(nav);
        }
    }

    fn render_body<'n>(
        &mut self,
        document: &mut Document,
        context: &super::RenderContext<'n>,
        parent: DomNode,
        token: &crate::lmarkdown::Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        match token {
            Token::OrderedList { items } => {
                let ol = document.create_element("ol");
                for tokens in items {
                    let li = document.create_element("li");
                    ol.append_child(li.clone());
                    tr.render(document, context, li, tokens);
                }
                parent.append_child(ol);
            }
            Token::BulletList { items } => {
                let ul = document.create_element("ul");
                for tokens in items {
                    let li = document.create_element("li");
                    ul.append_child(li.clone());
                    tr.render(document, context, li, tokens);
                }
                parent.append_child(ul);
            }
            Token::Attributes { .. } | Token::Comment { .. } => {}

            Token::ThematicBreak => {
                parent.append_child(document.create_element("hr"));
            }
            Token::Image { tokens, src } => {
                parent.append_child(document.create_element_with_attributes(
                    "img",
                    to_attributes([("src", src), ("alt", &tokens_to_text(tokens))]),
                ));
            }
            Token::BlockQuote { tokens } => {
                let blockquote = document.create_element("blockquote");
                tr.render(document, context, blockquote.clone(), tokens);
                parent.append_child(blockquote);
            }
            Token::HardBreak { .. } => {
                parent.append_child(document.create_element("br"));
            }
            Token::SoftBreak { .. } => {
                parent.append_child(document.create_text_node(" "));
            }
            Token::Heading { depth, tokens } => {
                let heading = document.create_element(format!("h{depth}"));
                tr.render(document, context, heading.clone(), tokens);
                parent.append_child(heading)
            }
            Token::Paragraph { tokens, .. } => {
                let p = document.create_element("p");
                tr.render(document, context, p.clone(), tokens);
                parent.append_child(p)
            }
            Token::Bold { text } => {
                let b = document.create_element("b");
                b.append_child(document.create_text_node(text));
                parent.append_child(b)
            }
            Token::Emphasis { text } => {
                let e = document.create_element("em");
                e.append_child(document.create_text_node(text));
                parent.append_child(e)
            }
            Token::Code {
                text: code,
                info: _,
            } => {
                let code_html = document.create_element("code");
                code_html.append_child(document.create_text_node(code));
                parent.append_child(code_html)
            }
            Token::Link { tokens: text, href } => {
                if text.len() == 0 {
                    return Some(parent);
                }

                // external link
                if is_href_external(href) {
                    let a = document
                        .create_element_with_attributes("a", to_attributes([("href", href)]));
                    tr.render(document, context, a.clone(), text);
                    parent.append_child(a);

                    parent.append_child(html!(<svg width="1em" height="1em" viewBox="0 0 24 24" style="cursor:pointer"><g stroke-width="2.1" stroke="#666" fill="none" stroke-linecap="round" stroke-linejoin="round"><polyline points="17 13.5 17 19.5 5 19.5 5 7.5 11 7.5"></polyline><path d="M14,4.5 L20,4.5 L20,10.5 M20,4.5 L11,13.5"></path></g></svg>));
                    return Some(parent);
                }

                if Page::is_href_to_page(href) {
                    let to_id = context
                        .site_tree
                        .links_from(context.site_id)
                        .into_iter()
                        .find_map(|l| {
                            if let Relation::Discovered { raw_path: path } = &l.relation {
                                if path == href {
                                    return Some(l.to);
                                }
                            }
                            None
                        });
                    if let Some(to_id) = to_id {
                        let rel_path = context.site_tree.path(to_id);
                        let a = document.create_element_with_attributes(
                            "a",
                            to_attributes([("href", rel_path)]),
                        );
                        tr.render(document, context, a.clone(), text);
                        parent.append_child(a.clone());
                        return Some(a);
                    }
                    warn!("Could not find node where {href:?} points to");
                }

                let a =
                    document.create_element_with_attributes("a", to_attributes([("href", href)]));
                tr.render(document, context, a.clone(), text);
                parent.append_child(a);
            }
            Token::Text { text } => {
                parent.append_child(document.create_text_node(text));
            }
            Token::Html {
                tag,
                attributes,
                tokens,
            } => {
                let p = render_html::render_html(
                    document, context, &parent, tr, tag, attributes, tokens,
                );
                if p.is_some() {
                    return p;
                }
            }
        };
        Some(parent)
    }

    fn after_render<'n>(&mut self, dom: &mut Document, _: &RenderContext<'n>) {
        let body = &dom.body;

        // move all dom elements to under #content
        let content = dom.create_element_with_attributes("div", to_attributes([("id", "content")]));
        for child in body.children() {
            child.detach();
            content.append_child(child);
        }
        body.append_child(content);

        // add watermark
        body.append_child(html!(<footer id="watermark">Generated by <a href="https://github.com/lyr-7D1h/lssg">LSSG</a></footer>));
    }
}

pub fn is_href_external(href: &str) -> bool {
    return href.starts_with("http") || href.starts_with("mailto:");
}

pub fn tokens_to_text(tokens: &Vec<Token>) -> String {
    let mut result = String::new();
    for t in tokens {
        if let Some(text) = t.to_text() {
            result.push_str(&text)
        }
    }
    return result;
}
