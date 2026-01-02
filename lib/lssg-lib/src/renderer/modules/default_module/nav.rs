use virtual_dom::{to_attributes, Document, DomNode};

use crate::renderer::RenderContext;

pub(super) fn breadcrumbs(document: &Document, ctx: &RenderContext) -> DomNode {
    let site_id = ctx.site_id;
    let site_tree = ctx.site_tree;
    let nav =
        document.create_element_with_attributes("nav", to_attributes([("class", "breadcrumbs")]));

    nav.append_child(document.create_text_node("/"));

    let parents = site_tree.parents(site_id);
    let parents_length = parents.len();
    for (i, p) in parents.into_iter().rev().enumerate() {
        let el = match &site_tree[p].kind {
            crate::sitetree::SiteNodeKind::Page(_) => document.create_element_with_attributes(
                "a",
                to_attributes([("href", site_tree.rel_path(site_id, p))]),
            ),
            crate::sitetree::SiteNodeKind::Folder => document.create_element("span"),
            _ => continue,
        };
        el.append_child(document.create_text_node(site_tree[p].name.clone()));
        nav.append_child(el);
        if i != parents_length - 1 {
            nav.append_child(document.create_text_node("/"));
        }
    }
    nav.append_child(document.create_text_node(format!("/{}", site_tree[site_id].name)));
    nav
}
