use std::collections::HashMap;

use log::warn;

use proc_html::html;
use virtual_dom::{to_attributes, Document, DomNode};

use crate::{
    lmarkdown::Token,
    renderer::{RenderContext, TokenRenderer},
    sitetree::{Page, Relation},
};

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
        "links" if attributes.contains_key("boxes") => {
            let links: DomNode = html!(<nav class="links"></nav>).into();
            parent.append_child(links.clone());
            for t in tokens {
                match t {
                    Token::Link { tokens, href } => {
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

                            match to_id {
                                Some(to_id) => context.site_tree.path(to_id),
                                None => {
                                    warn!("Could not find node where {href:?} points to");
                                    return Some(links);
                                }
                            }
                        } else {
                            href.into()
                        };

                        let a: DomNode = html!(<a href="{href}"><div class="box"></div></a>).into();
                        let div = a.first_child().unwrap();
                        tr.render(document, context, div, tokens);
                        links.append_child(a);
                    }
                    _ => {}
                }
            }
        }
        _ => {
            let element = document.create_element_with_attributes(tag, attributes.clone());
            tr.render(document, context, element.clone(), tokens);
            parent.append_child(element)
        }
    }

    None
}
