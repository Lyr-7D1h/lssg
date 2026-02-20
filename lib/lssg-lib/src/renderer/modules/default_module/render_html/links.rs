use std::collections::HashMap;

use log::{error, warn};

use proc_virtual_dom::dom;
use virtual_dom::{Document, DomNode};

use crate::{
    lmarkdown::Token,
    renderer::{
        RenderContext, TokenRenderer, modules::default_module::translate_href_to_sitetree_path,
        util::tokens_to_text,
    },
};

fn links_grid(
    document: &mut Document,
    ctx: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    _attributes: &HashMap<String, String>,
    tokens: &[Token],
) {
    let links: Vec<DomNode> = tokens
        .iter()
        .filter_map(|t| {
            if let Token::Link { tokens, href, .. } = t {
                let links: Vec<_> = translate_href_to_sitetree_path(
                    href,
                    ctx.site_tree,
                    ctx.site_id,
                )
                .into_iter()
                .map(|(href, _)| {
                    let a = dom!(<a href="{href}"><div class="default__links_grid_card"></div></a>);
                    let mut tokens = tokens.iter().peekable();
                    // if link content starts with image use it as cover
                    if let Some(first) = tokens.peek()
                        && let Token::Image { .. } = first
                    {
                        let first = tokens.next().unwrap();
                        let cover = dom!(<div class="default__links_grid_card_cover"></div>);
                        let s =
                            tr.render(document, ctx, cover.clone(), std::slice::from_ref(first));
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
                    let tokens = Vec::from_iter(tokens.cloned());
                    let title = tokens_to_text(&tokens);
                    a.first_child().unwrap().append_child(
                        dom!(<h3 class="default__links_grid_card_title">{title}</h3>),
                    );
                    a
                })
                .collect();

                Some(links)
            } else {
                None
            }
        })
        .flatten()
        .collect();
    let grid: DomNode = dom!(<div class="default__links_grid">{links}</div>);
    parent.append_child(grid);
}

fn links_boxes(
    document: &mut Document,
    ctx: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    _attributes: &HashMap<String, String>,
    tokens: &[Token],
) {
    let links: DomNode = dom!(<nav class="default__links"></nav>);
    parent.append_child(links.clone());
    for t in tokens {
        if let Token::Link {
            tokens,
            href,
            title,
        } = t
        {
            for (href, _) in translate_href_to_sitetree_path(href, ctx.site_tree, ctx.site_id) {
                let a: DomNode = if let Some(title) = title {
                    dom!(<a href="{href}" title="{title}"><div class="default__links_box"></div></a>)
                } else {
                    dom!(<a href="{href}"><div class="default__links_box"></div></a>)
                };

                let div = a.first_child().unwrap();
                tr.render(document, ctx, div, tokens);
                links.append_child(a);
            }
        }
    }
}

pub fn links(
    document: &mut Document,
    context: &RenderContext,
    parent: &DomNode,
    tr: &mut TokenRenderer,
    attributes: &HashMap<String, String>,
    tokens: &[Token],
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
