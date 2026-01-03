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
    sitetree::{Page, Relation, SiteId},
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
    let links: DomNode = dom!(<nav class="default__links"></nav>).into();
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
                    dom!(<a href="{href}" title="{title}"><div class="default__links_box"></div></a>).into()
                } else {
                    dom!(<a href="{href}"><div class="default__links_box"></div></a>).into()
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

    let item =
        || dom!(<div class="default__carausel_item" onclick="default__toggleModal(event)"></div>);
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

pub fn sitetree(ctx: &RenderContext, parent: &DomNode, attributes: &HashMap<String, String>) {
    let ignore_list: Vec<&str> = attributes
        .get("ignore")
        .map(|s| s.split(',').collect())
        .unwrap_or(vec![]);

    let mut map = ctx.site_tree.flatten_to_pages();

    // Filter out ignored nodes
    if !ignore_list.is_empty() {
        for node_children in &mut map {
            node_children.retain(|child_id| {
                let name = ctx.site_tree[*child_id].name.as_str();
                !ignore_list.contains(&name)
            });
        }
    }

    let tree = sitetree_recurs(ctx.site_tree.root(), &map, ctx, true);
    let sitetree = dom!(<div class="default__sitetree">{tree}</div>);
    parent.append_child(sitetree);
}

fn sitetree_recurs(
    id: SiteId,
    map: &Vec<Vec<SiteId>>,
    ctx: &RenderContext,
    root: bool,
) -> Vec<DomNode> {
    let mut children = map[*id].clone();
    children.sort_by(|a, b| {
        let (a, b) = (*a, *b);
        let a_name = &ctx.site_tree[a].name;
        let b_name = &ctx.site_tree[b].name;
        let a_has_children = map[*a].len() > 0;
        let b_has_children = map[*b].len() > 0;

        // Sort by has_children first (true before false), then by name
        b_has_children
            .cmp(&a_has_children)
            .then_with(|| a_name.cmp(&b_name))
    });
    let children: Vec<DomNode> = children
        .into_iter()
        .flat_map(|c| sitetree_recurs(c, map, ctx, false))
        .collect();

    if root {
        return children;
    }

    let node = &ctx.site_tree[id];
    let name = &node.name;
    let has_children = children.len() > 0;
    let name = format!("{name}{}", if has_children { "/" } else { "" });
    let path = ctx.site_tree.path(id);
    if has_children {
        vec![dom!(
            <div class="default__sitetree_folder">
                <a href="{path}">{name}</a>
                <div class="default__sitetree_folder_content">
                    {children}
                </div>
            </div>
        )]
    } else {
        vec![dom!(<div class="default__sitetree_file"><a href="{path}">{name}</a></div>)]
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
                to_attributes([("class", "default__centered")]),
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
