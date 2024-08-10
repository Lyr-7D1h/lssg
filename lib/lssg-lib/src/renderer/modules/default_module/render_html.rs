use std::collections::HashMap;

use log::{error, warn};

use proc_virtual_dom::dom;
use virtual_dom::{to_attributes, Document, DomNode};

use crate::{
    lmarkdown::Token,
    renderer::{
        util::{process_href, tokens_to_text},
        RenderContext, TokenRenderer,
    },
    sitetree::{Page, Relation},
    tree::Node,
};

fn links_grid(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    _attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    let links: Vec<DomNode> = tokens
        .into_iter()
        .filter_map(|t| {
            if let Token::Link { tokens, href, .. } = t {
                let href = process_href(href, context);
                let a = dom!(<a href="{href}"><div class="default__links_grid_card"></div></a>);
                let mut tokens = tokens.iter().peekable();
                // if link content starts with image use it as cover
                if let Some(first) = tokens.peek() {
                    if let Token::Image { .. } = first {
                        let first = tokens.next().unwrap();
                        let cover = dom!(<div class="default__links_grid_card_cover"></div>);
                        let s = tr.render(document, context, cover.clone(), &vec![first.clone()]);
                        // if svg set viewbox to allow scaling
                        match &mut *s.first_child().unwrap().kind_mut() {
                            virtual_dom::DomNodeKind::Element { attributes, .. } => {
                                attributes.insert("width".into(), "100%".into());
                                attributes.insert("height".into(), "auto".into());
                            }
                            _ => error!("should be an element"),
                        }
                        a.first_child().unwrap().append_child(cover);
                    }
                }
                let tokens = Vec::from_iter(tokens.cloned());
                let title = tokens_to_text(&tokens);
                a.first_child()
                    .unwrap()
                    .append_child(dom!(<h2 class="default__links_grid_card_title">{title}</h2>));

                Some(a)
            } else {
                None
            }
        })
        .collect();
    let grid: DomNode = dom!(<div class="default__links_grid">{links}</div>).into();
    parent.append_child(grid);
}

fn links_boxes(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    _attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    let links: DomNode = dom!(<nav class="links"></nav>).into();
    parent.append_child(links.clone());
    for t in tokens {
        match t {
            Token::Link {
                tokens,
                href,
                title,
            } => {
                let mut href = href.clone();

                // if a local link to a page get site path to the page
                if Page::is_href_to_page(&href) {
                    // find site id where href is pointing to
                    let to_id = context
                        .site_tree
                        .links_from(context.site_id)
                        .into_iter()
                        .find_map(|l| {
                            if let Relation::Discovered { raw_path: path } = &l.relation {
                                if *path == href {
                                    return Some(l.to);
                                }
                            }
                            None
                        });

                    href = match to_id {
                        Some(to_id) => context.site_tree.path(to_id),
                        None => {
                            warn!("Could not find node where {href:?} points to, ignoring..");
                            continue;
                        }
                    }
                }

                let a: DomNode = if let Some(title) = title {
                    dom!(<a href="{href}" title="{title}"><div class="box"></div></a>).into()
                } else {
                    dom!(<a href="{href}"><div class="box"></div></a>).into()
                };

                let div = a.first_child().unwrap();
                tr.render(document, context, div, tokens);
                links.append_child(a);
            }
            _ => {}
        }
    }
}

fn links(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    if attributes.contains_key("boxes") {
        links_boxes(document, context, parent, tr, attributes, tokens);
    } else if attributes.contains_key("grid") {
        links_grid(document, context, parent, tr, attributes, tokens);
    } else {
        warn!("unknown links html element, ignoring..");
        tr.render(document, context, parent.clone(), tokens);
    }
}

fn carousel(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    _attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    let carousel = dom!(<div class="default__carausel"></div>);

    let main = dom!(<div class="default__carausel_main"></div>);
    let mut tokens = tokens.into_iter();

    let item = || dom!(<div class="default__carausel_item" ></div>);
    match tokens.next() {
        Some(t) => {
            let item = item();
            tr.render(document, context, item.clone(), &vec![t.clone()]);
            main.append_child(item);
        }
        None => return,
    }
    carousel.append_child(main);

    if tokens.len() > 0 {
        let items: Vec<DomNode> = tokens
            .map(|t| {
                let item = item();
                tr.render(document, context, item.clone(), &vec![t.clone()]);
                item
            })
            .collect();
        carousel.append_child(dom!(<div class="default__carausel_other">{items}</div>))
    }

    parent.append_child(carousel);
}

pub fn sitetree(context: &RenderContext, parent: &DomNode, attributes: &HashMap<String, String>) {
    let ignore_list = attributes
        .get("ignore")
        .map(|s| s.split(',').collect())
        .unwrap_or(vec![]);

    let sitetree = dom!(<div class="default__sitetree"></div>);
    for c in context.site_tree[context.site_tree.root()].children() {
        if let Some(child) = sitetree_recurs(*c, context, &ignore_list) {
            sitetree.append_child(child)
        }
    }
    parent.append_child(sitetree);
}

fn sitetree_recurs(id: usize, context: &RenderContext, ignore_list: &Vec<&str>) -> Option<DomNode> {
    let node = &context.site_tree[id];
    let name = &node.name;
    if ignore_list.contains(&name.as_str()) {
        return None;
    }
    let children: Vec<DomNode> = context.site_tree[id]
        .children()
        .into_iter()
        .filter_map(|c| sitetree_recurs(*c, context, ignore_list))
        .collect();
    let n = match &node.kind {
        crate::sitetree::SiteNodeKind::Page(_) => {
            let slash = if children.len() > 0 { "/" } else { "" };
            let name = format!("{}{slash}", &node.name);
            let path = context.site_tree.path(id);
            dom!(<div class="default__sitetree_page"><a href="{path}">{name}</a>{children}</div>)
        }
        _ => return None,
    };
    return Some(n);
}

pub fn render_html(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    tag: &str,
    attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) -> Option<DomNode> {
    match tag {
        "centered" => {
            let centered = DomNode::create_element_with_attributes(
                "div",
                to_attributes([("class", "centered")]),
            );
            tr.render(document, context, centered.clone(), tokens);
            parent.append_child(centered);
        }
        "links" => links(document, context, parent, tr, attributes, tokens),
        "sitetree" => sitetree(context, parent, attributes),
        "carousel" => carousel(document, context, parent, tr, attributes, tokens),
        _ => {
            let element = document.create_element_with_attributes(tag, attributes.clone());
            tr.render(document, context, element.clone(), tokens);
            parent.append_child(element)
        }
    }

    None
}
