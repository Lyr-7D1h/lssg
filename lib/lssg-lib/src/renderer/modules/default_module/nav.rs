use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_extensions::Overwrite;
use virtual_dom::{Document, DomNode, to_attributes};

use crate::{renderer::RenderContext, sitetree::SiteId};

use super::PropegatedOptionsWithRoot;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NavKind {
    Breadcrumbs,
    #[serde(rename = "sidemenu")]
    SideMenu,
    None,
}
impl Overwrite for NavKind {
    fn overwrite<'de, D>(&mut self, d: D) -> Result<(), D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        *self = Deserialize::deserialize(d)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Overwrite)]
pub(super) struct NavOptions {
    kind: NavKind,
    include_root: bool,
    use_global_root: bool,
    ignore: Vec<String>,
    name_map: HashMap<String, String>,
}

impl Default for NavOptions {
    fn default() -> Self {
        Self {
            kind: NavKind::Breadcrumbs,
            include_root: true,
            use_global_root: false,
            ignore: Vec::new(),
            name_map: HashMap::new(),
        }
    }
}

fn breadcrumbs(
    document: &Document,
    ctx: &RenderContext,
    root_site_id: crate::sitetree::SiteId,
    include_root: bool,
    ignore: &[String],
) -> DomNode {
    let site_id = ctx.site_id;
    let site_tree = ctx.site_tree;
    let nav = document
        .create_element_with_attributes("nav", to_attributes([("class", "default__breadcrumbs")]));

    nav.append_child(document.create_text_node("/"));

    let parents: Vec<SiteId> = site_tree.parents(site_id).collect();

    // Filter parents to only include those up to (and including) the root_site_id
    let mut filtered_parents = parents
        .into_iter()
        .rev()
        .skip_while(|p| *p != root_site_id)
        .peekable();

    if !include_root {
        filtered_parents.next();
    }

    while let Some(p) = filtered_parents.next() {
        // Skip ignored nodes
        if ignore.contains(&site_tree[p].name()) {
            continue;
        }
        let el = match &site_tree[p].kind {
            crate::sitetree::SiteNodeKind::Page(_) => document.create_element_with_attributes(
                "a",
                to_attributes([("href", site_tree.rel_path(site_id, p))]),
            ),
            crate::sitetree::SiteNodeKind::Folder => document.create_element("span"),
            _ => continue,
        };
        el.append_child(document.create_text_node(site_tree[p].name().clone()));
        nav.append_child(el);
        if filtered_parents.peek().is_some() {
            nav.append_child(document.create_text_node("/"));
        }
    }
    nav.append_child(document.create_text_node(format!("/{}", site_tree[site_id].name())));
    nav
}

fn format_node_name(name: &str, name_map: &HashMap<String, String>) -> String {
    name_map
        .get(name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| name.to_string())
}

fn side_menu(
    document: &Document,
    ctx: &RenderContext,
    root_site_id: crate::sitetree::SiteId,
    include_root: bool,
    ignore: &[String],
    name_map: &HashMap<String, String>,
) -> DomNode {
    let site_id = ctx.site_id;
    let site_tree = ctx.site_tree;

    let nav = document
        .create_element_with_attributes("nav", to_attributes([("class", "default__side-menu")]));

    // Get flattened page hierarchy
    let map = site_tree.flatten_to_pages();

    // Use the provided root_site_id instead of site_tree.root()
    let root_id = root_site_id;

    if include_root {
        let root_node = &site_tree[root_id];
        let root_ul = document.create_element("ul");
        let root_li = document.create_element("li");

        let is_current = root_id == site_id;
        let mut classes = vec!["default__side-menu__link", "default__side-menu__root"];
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
        let formatted_name = format_node_name(&root_node.name(), name_map);
        root_link.append_child(document.create_text_node(formatted_name));
        root_li.append_child(root_link);

        // Add children
        if !map[*root_id].is_empty() {
            let menu_list = build_menu_tree(
                document, site_tree, &map, root_id, site_id, 0, ignore, name_map,
            );
            root_li.append_child(menu_list);
        }

        root_ul.append_child(root_li);
        nav.append_child(root_ul);
    } else {
        let menu_list = build_menu_tree(
            document, site_tree, &map, root_id, site_id, 0, ignore, name_map,
        );
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
    ignore: &[String],
    name_map: &HashMap<String, String>,
) -> DomNode {
    let ul = document.create_element("ul");

    // Get page children from the flattened map
    for child_id in map[*node_id].iter() {
        let child = &site_tree[*child_id];

        // Skip ignored nodes
        if ignore.contains(&child.name()) {
            continue;
        }

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
        let formatted_name = format_node_name(&child.name(), name_map);
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
                ignore,
                name_map,
            );
            li.append_child(nested_menu);
        }

        ul.append_child(li);
    }

    ul
}

pub fn nav(opts_wrapper: &PropegatedOptionsWithRoot, document: &mut Document, ctx: &RenderContext) {
    for opt in opts_wrapper.options.nav.as_slice().iter() {
        // Use root_site_id from the wrapper or fall back to the site tree root
        let root_id = if opt.use_global_root {
            ctx.site_tree.root()
        } else {
            opts_wrapper
                .root_site_id
                .unwrap_or_else(|| ctx.site_tree.root())
        };

        let ignore = &opt.ignore;
        let name_map = &opt.name_map;
        let include_root = opt.include_root;
        let el = match opt.kind {
            NavKind::None => continue,
            NavKind::Breadcrumbs => {
                // never show root on breadcrumbs
                if ctx.site_id == root_id {
                    continue;
                }
                breadcrumbs(document, ctx, root_id, include_root, ignore)
            }
            NavKind::SideMenu => side_menu(document, ctx, root_id, include_root, ignore, name_map),
        };

        document.body.prepend(el);
    }
}
