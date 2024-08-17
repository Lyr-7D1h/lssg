use std::collections::HashMap;

use log::{error, warn};

use proc_virtual_dom::dom;
use regex::Regex;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    lssg_error::LssgError,
    sitetree::{Input, Page, Relation, Resource, SiteNode, SiteNodeKind, SiteTree, Stylesheet},
    tree::DFS,
};
use virtual_dom::{self, parse_html, to_attributes, Document, DomNode, DomNodeKind, Html};

use crate::renderer::{RenderContext, RendererModule, TokenRenderer};

use super::util::{process_href, tokens_to_text};

mod render_html;

const DEFAULT_STYLESHEET: &[u8] = include_bytes!("./default_stylesheet.css");
const DEFAULT_JS: &str = include_str!("./default.js");

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

/// Render everything meant to go into <head>
fn head(document: &mut Document, context: &RenderContext, options: &PropegatedOptions) {
    let RenderContext {
        site_id,
        site_tree,
        page,
        ..
    } = context;
    let site_id = *site_id;

    let head = &document.head;
    let mut title = options.title.clone();
    if let Some(header) = page
        .tokens()
        .iter()
        .find(|t| {
            if let Token::Heading { depth, .. } = t {
                return *depth == 1;
            }
            false
        })
        .cloned()
    {
        let header = tokens_to_text(&vec![header]);
        title = format!("{header} - {title}");
    }

    head.append_child(dom!(<title>{title}</title>));
    let title = options.title.clone();
    head.append_child(dom!(<meta property="og:title" content="{title}" />));
    let title = options.title.clone();
    head.append_child(dom!(<meta name="twitter:title" content="{title}" />));

    // add stylesheets and favicon
    // reverse the order of insertion because latest css is applied last
    for link in site_tree.links_from(site_id).into_iter().rev() {
        match link.relation {
            Relation::External | Relation::Discovered { .. } => match site_tree[link.to].kind {
                SiteNodeKind::Resource { .. } if site_tree[link.to].name == "favicon.ico" => {
                    head.append_child(document.create_element_with_attributes(
                        "link",
                        to_attributes([
                            ("rel", "icon"),
                            ("type", "image/x-icon"),
                            ("href", &site_tree.rel_path(site_id, link.to)),
                        ]),
                    ));
                }
                SiteNodeKind::Resource { .. } if site_tree[link.to].name.ends_with("js") => {
                    let path = &site_tree.rel_path(site_id, link.to);
                    document
                        .body
                        .append_child(dom!(<script src="{path}"></script>));
                }
                SiteNodeKind::Stylesheet { .. } => {
                    head.append_child(document.create_element_with_attributes(
                        "link",
                        to_attributes([
                            ("rel", "stylesheet"),
                            ("href", &site_tree.rel_path(site_id, link.to)),
                        ]),
                    ));
                }
                _ => {}
            },
            _ => {}
        }
    }

    // meta tags
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
        if key == "description" {
            head.append_child(document.create_element_with_attributes(
                "meta",
                to_attributes([("name", key), ("content", value)]),
            ));
        }
        match key.as_str() {
            "description" | "image" => {
                head.append_child(document.create_element_with_attributes(
                    "meta",
                    to_attributes([("property", &format!("og:{}", key)), ("content", value)]),
                ));
                head.append_child(document.create_element_with_attributes(
                    "meta",
                    to_attributes([("name", &format!("twitter:{}", key)), ("content", value)]),
                ));
                continue;
            }
            _ => {}
        }
        // Open Graph (https://ogp.me/) uses property instead of name
        if key.starts_with("og:") {
            head.append_child(document.create_element_with_attributes(
                "meta",
                to_attributes([("property", key), ("content", value)]),
            ));
        } else {
            head.append_child(document.create_element_with_attributes(
                "meta",
                to_attributes([("name", key), ("content", value)]),
            ));
        }
    }
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
        let pages: Vec<usize> = DFS::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();

        let default_js = site_tree.add(SiteNode::resource(
            "default.js",
            site_tree.root(),
            Resource::new_static(DEFAULT_JS.to_owned()),
        ));
        site_tree.add_link(site_tree.root(), default_js);

        let default_stylesheet = site_tree.add(SiteNode::stylesheet(
            "default.css",
            site_tree.root(),
            Stylesheet::from_readable(DEFAULT_STYLESHEET)?,
        ));
        site_tree.add_link(site_tree.root(), default_stylesheet);

        let mut relation_map: HashMap<usize, Vec<usize>> = HashMap::new();
        // propegate relations to stylesheets and favicon from parent to child
        for id in pages {
            // skip page if disabled
            if let SiteNodeKind::Page(page) = &site_tree[id].kind {
                let opts: SinglePageOptions = self.options(page);
                if opts.root {
                    continue;
                }
            }

            // get the set of links to favicon and stylesheets
            let mut set: Vec<usize> = site_tree
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
                    // 0 [28, 29, 32, 35, 37, 49]
                    // add links from parent_set without the ones it already has
                    let mut new_links: Vec<usize> = parent_set
                        .into_iter()
                        .filter(|id| !set.contains(id))
                        .cloned()
                        .collect();
                    for to in new_links.iter() {
                        site_tree.add_link(id, *to);
                    }
                    new_links.extend(set.iter());
                    set = new_links;
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

    fn after_render<'n>(&mut self, document: &mut Document, context: &RenderContext<'n>) {
        let site_id = context.site_id;
        let site_tree = context.site_tree;
        let body = &document.body;

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

            body.prepend(nav);
        }

        // move all dom elements to under #content
        let content =
            document.create_element_with_attributes("div", to_attributes([("id", "content")]));
        for child in body.children() {
            child.detach();
            content.append_child(child);
        }
        body.append_child(content);

        // add watermark
        body.append_child(dom!(<footer id="watermark">Generated by <a href="https://github.com/lyr-7D1h/lssg">LSSG</a></footer>));

        let options = self
            .options_map
            .get(&site_id)
            .expect("expected options map to contain all page ids");

        // Add language to html tag
        if let DomNodeKind::Element { attributes, .. } = &mut *document.root().kind_mut() {
            attributes.insert("lang".to_owned(), options.language.clone());
        }

        // fill head
        head(document, context, options);
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
            Token::OrderedList { items, .. } => {
                let ol = document.create_element("ol");
                for tokens in items {
                    let li = document.create_element("li");
                    ol.append_child(li.clone());
                    // don't render paragraphs inside of lists
                    let tokens = tokens
                        .into_iter()
                        .flat_map(|t| {
                            if let Token::Paragraph { tokens, .. } = t {
                                return tokens.clone();
                            }
                            vec![t.clone()]
                        })
                        .collect();
                    tr.render(document, context, li, &tokens);
                }
                parent.append_child(ol);
            }
            Token::BulletList { items, .. } => {
                let ul = document.create_element("ul");
                for tokens in items {
                    let li = document.create_element("li");
                    ul.append_child(li.clone());
                    // don't render paragraphs inside of lists
                    let tokens = tokens
                        .into_iter()
                        .flat_map(|t| {
                            if let Token::Paragraph { tokens, .. } = t {
                                return tokens.clone();
                            }
                            vec![t.clone()]
                        })
                        .collect();
                    tr.render(document, context, li, &tokens);
                }
                parent.append_child(ul);
            }
            Token::Attributes { .. } | Token::Comment { .. } => {}

            Token::ThematicBreak => {
                parent.append_child(document.create_element("hr"));
            }
            Token::Image { tokens, src, title } => {
                let mut resource_id = None;
                // if local page return relative src
                let src = if Input::is_relative(src) {
                    let to_id = context
                        .site_tree
                        .links_from(context.site_id)
                        .into_iter()
                        .find_map(|l| {
                            if let Relation::Discovered { raw_path: path } = &l.relation {
                                if path == src {
                                    return Some(l.to);
                                }
                            }
                            None
                        });

                    if let Some(to_id) = to_id {
                        resource_id = Some(to_id);
                        context.site_tree.path(to_id)
                    } else {
                        warn!("Could not find node where {src:?} points to");
                        src.to_owned()
                    }
                } else {
                    src.to_owned()
                };

                // inject svg into html
                if src.ends_with(".svg") {
                    let readable = if let Some(id) = resource_id {
                        match &context.site_tree[id].kind {
                            SiteNodeKind::Resource(r) => r.readable(),
                            _ => {
                                warn!("svg is not found as a resource");
                                return Some(parent);
                            }
                        }
                    } else {
                        match Input::from_string(&src) {
                            Ok(i) => i.readable(),
                            Err(e) => {
                                error!("failed to get svg: {e}");
                                return Some(parent);
                            }
                        }
                    };

                    match readable {
                        Ok(r) => {
                            // get first valid html tag
                            let mut html = parse_html(r)
                                .unwrap()
                                .into_iter()
                                .find(|e| match e {
                                    Html::Comment { .. } | Html::Text { .. } => false,
                                    Html::Element { .. } => true,
                                })
                                .expect("invalid svg, no html elements founds");

                            match &mut html {
                                Html::Element { attributes, .. } => {
                                    // set viewbox to allow scaling of svg using width and height
                                    if attributes.get("viewbox").is_none() {
                                        let re = Regex::new(r"[0-9]*").unwrap();
                                        let width = if let Some(width) = attributes.get("width") {
                                            re.captures(width)
                                                .map(|c| c[0].to_string())
                                                .unwrap_or(width.clone())
                                        } else {
                                            warn!("no width found for svg, using default of 300");
                                            "300".to_string()
                                        };
                                        let height = if let Some(height) = attributes.get("height")
                                        {
                                            re.captures(height)
                                                .map(|c| c[0].to_string())
                                                .unwrap_or(width.clone())
                                        } else {
                                            warn!("no height found for svg, using default of 150");
                                            "150".to_string()
                                        };
                                        attributes.insert(
                                            "viewbox".into(),
                                            format!("0 0 {width} {height}"),
                                        );
                                    }
                                    attributes.remove(&"style".to_string());
                                    attributes.remove(&"width".to_string());
                                    attributes.remove(&"height".to_string());

                                    parent.append_child(html);
                                    return Some(parent);
                                }
                                _ => error!("svg must be an element"),
                            }
                        }

                        Err(e) => {
                            error!("failed to read {src}: {e}");
                            return Some(parent);
                        }
                    }
                }

                if src.ends_with(".mp4") {
                    parent.append_child(
                        dom!(<video controls><source src="{src}" type="video/mp4"></video>),
                    );
                    return Some(parent);
                }

                let alt = tokens_to_text(tokens);
                #[allow(unused_variables)]
                if let Some(title) = title {
                    parent.append_child(dom!(<img src="{src}" alt="{alt}" title={title} />))
                } else {
                    parent.append_child(dom!(<img src="{src}" alt="{alt}" />))
                }
            }
            Token::BlockQuote { tokens, .. } => {
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
            Token::Heading { depth, tokens, .. } => {
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
            Token::Link {
                tokens,
                href,
                title,
            } => {
                // ignore link if there is no text
                if tokens.len() == 0 {
                    return Some(parent);
                }

                let href = &process_href(href, context);
                // if local page return relative path
                let href = if Page::is_href_to_page(href) {
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
                        rel_path
                    } else {
                        warn!("Could not find node where {href:?} points to");
                        href.to_owned()
                    }
                } else {
                    href.to_owned()
                };

                let mut attributes = to_attributes([("href", href)]);
                if let Some(title) = title {
                    attributes.insert("title".to_owned(), title.to_owned());
                }
                let a = document.create_element_with_attributes("a", attributes);
                tr.render(document, context, a.clone(), tokens);
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
                if let Some(parent) = render_html::render_html(
                    document, context, &parent, tr, tag, attributes, tokens,
                ) {
                    return Some(parent);
                }
            }
        };
        // always renders
        Some(parent)
    }
}
