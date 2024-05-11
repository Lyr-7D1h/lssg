use std::collections::HashMap;

use log::{error, warn};

use proc_virtual_dom::dom;
use regex::Regex;
use virtual_dom::{to_attributes, Document, DomNode};

use crate::{
    lmarkdown::Token,
    renderer::{util::tokens_to_text, RenderContext, TokenRenderer},
    sitetree::{Page, Relation},
};

fn links_grid(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    tag: &str,
    attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    let links: Vec<DomNode> = tokens
        .into_iter()
        .filter_map(|t| {
            if let Token::Link { tokens, href, .. } = t {
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
                            virtual_dom::DomNodeKind::Element { attributes, tag } => {
                                if tag == "svg" {
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
                                }
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
    // tr.render(document, context, grid.clone(), tokens);
    parent.append_child(grid);
}

fn links_boxes(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    tag: &str,
    attributes: &HashMap<String, String>,
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
    tag: &str,
    attributes: &HashMap<String, String>,
    tokens: &Vec<Token>,
) {
    if attributes.contains_key("boxes") {
        links_boxes(document, context, parent, tr, tag, attributes, tokens);
    } else if attributes.contains_key("grid") {
        links_grid(document, context, parent, tr, tag, attributes, tokens);
    } else {
        warn!("unknown links html element, ignoring..");
        tr.render(document, context, parent.clone(), tokens);
    }
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
        "links" => links(document, context, parent, tr, tag, attributes, tokens),
        _ => {
            let element = document.create_element_with_attributes(tag, attributes.clone());
            tr.render(document, context, element.clone(), tokens);
            parent.append_child(element)
        }
    }

    None
}
