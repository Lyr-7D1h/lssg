use std::collections::HashMap;

use proc_virtual_dom::dom;
use virtual_dom::{Document, DomNode, to_attributes};

use crate::{
    lmarkdown::Token,
    renderer::{RenderContext, TokenRenderer},
    sitetree::SiteId,
};

mod carousel;
mod links;

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
                let name = ctx.site_tree[*child_id].name().as_str();
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
        let a_name = &ctx.site_tree[a].name();
        let b_name = &ctx.site_tree[b].name();
        let a_has_children = !map[*a].is_empty();
        let b_has_children = !map[*b].is_empty();

        // Sort by has_children first (true before false), then by name
        b_has_children
            .cmp(&a_has_children)
            .then_with(|| a_name.cmp(b_name))
    });
    let children: Vec<DomNode> = children
        .into_iter()
        .flat_map(|c| sitetree_recurs(c, map, ctx, false))
        .collect();

    if root {
        return children;
    }

    let node = &ctx.site_tree[id];
    let name = &node.name();
    let has_children = !children.is_empty();
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
    tokens: &[Token],
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
        "links" => links::links(document, context, parent, tr, attributes, tokens),
        "sitetree" => sitetree(context, parent, attributes),
        "carousel" => carousel::carousel(document, context, parent, tr, attributes, tokens),
        _ => {
            let element = document.create_element_with_attributes(tag, attributes.clone());
            tr.render(document, context, element.clone(), tokens);
            parent.append_child(element)
        }
    }

    None
}
