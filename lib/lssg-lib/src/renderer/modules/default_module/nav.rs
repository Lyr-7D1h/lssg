use std::collections::HashMap;

use serde::Deserialize;
use serde_extensions::Overwrite;
use virtual_dom::{to_attributes, Document, DomNode};

use crate::renderer::RenderContext;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavKind {
    Breadcrumbs,
    #[serde(rename = "sidemenu")]
    SideMenu,
    None,
}

#[derive(Debug, Clone, Deserialize, Overwrite)]
pub(super) struct NavOptions {
    kind: Option<NavKind>,
    include_root: Option<bool>,
    name_map: Option<HashMap<String, String>>,
}

impl Default for NavOptions {
    fn default() -> Self {
        Self {
            kind: Some(NavKind::Breadcrumbs),
            include_root: Some(false),
            name_map: None,
        }
    }
}

fn breadcrumbs(document: &Document, ctx: &RenderContext) -> DomNode {
    let site_id = ctx.site_id;
    let site_tree = ctx.site_tree;
    let nav = document
        .create_element_with_attributes("nav", to_attributes([("class", "default__breadcrumbs")]));

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

fn format_node_name(name: &str, name_map: Option<&HashMap<String, String>>) -> String {
    // Check if there's a custom mapping for this name
    if let Some(map) = name_map {
        map.get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string())
    } else {
        name.to_string()
    }
}

fn side_menu(
    document: &Document,
    ctx: &RenderContext,
    include_root: bool,
    name_map: Option<&HashMap<String, String>>,
) -> DomNode {
    let site_id = ctx.site_id;
    let site_tree = ctx.site_tree;

    let nav = document
        .create_element_with_attributes("nav", to_attributes([("class", "default__side-menu")]));

    // Get flattened page hierarchy
    let map = site_tree.flatten_to_pages();

    // Build a hierarchical menu structure
    let root_id = site_tree.root();

    // If include_root is true, wrap the entire menu with a root link
    if include_root {
        let root_node = &site_tree[root_id];
        let root_ul = document.create_element("ul");
        let root_li = document.create_element("li");

        let is_current = root_id == site_id;
        let mut classes = vec!["default__side-menu__link"];
        if is_current {
            classes.push("default__side-menu__link--active");
        }

        let root_link = document.create_element_with_attributes(
            "a",
            to_attributes([
                ("href", site_tree.rel_path(site_id, root_id)),
                ("class", classes.join(" ")),
            ]),
        );
        let formatted_name = format_node_name(&root_node.name, name_map);
        root_link.append_child(document.create_text_node(formatted_name));
        root_li.append_child(root_link);

        // Add children
        if !map[*root_id].is_empty() {
            let menu_list =
                build_menu_tree(document, site_tree, &map, root_id, site_id, 0, name_map);
            root_li.append_child(menu_list);
        }

        root_ul.append_child(root_li);
        nav.append_child(root_ul);
    } else {
        let menu_list = build_menu_tree(document, site_tree, &map, root_id, site_id, 0, name_map);
        nav.append_child(menu_list);
    }

    nav
}

fn build_menu_tree(
    document: &Document,
    site_tree: &crate::sitetree::SiteTree,
    map: &Vec<Vec<crate::sitetree::SiteId>>,
    node_id: crate::sitetree::SiteId,
    current_id: crate::sitetree::SiteId,
    depth: usize,
    name_map: Option<&HashMap<String, String>>,
) -> DomNode {
    let ul = document.create_element("ul");

    // Get page children from the flattened map
    for child_id in map[*node_id].iter().rev() {
        let child = &site_tree[*child_id];
        let li = document.create_element("li");

        // Check if this is the current page
        let is_current = *child_id == current_id;

        // Create the link
        let mut classes = vec!["default__side-menu__link"];
        if is_current {
            classes.push("default__side-menu__link--active");
        }

        let a = document.create_element_with_attributes(
            "a",
            to_attributes([
                ("href", site_tree.rel_path(current_id, *child_id)),
                ("class", classes.join(" ")),
            ]),
        );
        let formatted_name = format_node_name(&child.name, name_map);
        a.append_child(document.create_text_node(formatted_name));
        li.append_child(a);

        // Recursively add all children if this node has any and depth limit not reached
        if !map[**child_id].is_empty() && depth < 3 {
            let nested_menu = build_menu_tree(
                document,
                site_tree,
                map,
                *child_id,
                current_id,
                depth + 1,
                name_map,
            );
            li.append_child(nested_menu);
        }

        ul.append_child(li);
    }

    ul
}

pub fn nav(opts: NavOptions, document: &mut Document, ctx: &RenderContext) {
    if let Some(kind) = opts.kind {
        let el = match kind {
            NavKind::None => return,
            NavKind::Breadcrumbs => {
                // don't show breadcrumbs on root
                if ctx.site_id == ctx.site_tree.root() {
                    return;
                }
                breadcrumbs(document, ctx)
            }
            NavKind::SideMenu => {
                let include_root = opts.include_root.unwrap_or(false);
                let name_map = opts.name_map.as_ref();
                side_menu(document, ctx, include_root, name_map)
            }
        };
        document.body.prepend(el);
    }
}
