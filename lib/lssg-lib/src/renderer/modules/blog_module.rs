use std::collections::HashSet;

use chrono::{DateTime, Utc};

use crate::{
    html::{to_attributes, DomTree},
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::SiteNodeKind,
    tree::DFS,
};

use super::{RendererModule, TokenRenderer};

pub struct BlogModule {
    enabled_site_ids: HashSet<usize>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            enabled_site_ids: HashSet::new(),
            has_inserted_date: false,
        }
    }
}

impl RendererModule for BlogModule {
    fn id(&self) -> &'static str {
        return "blog";
    }

    fn after_init(
        &mut self,
        site_tree: &crate::sitetree::SiteTree,
    ) -> Result<(), crate::lssg_error::LssgError> {
        // if parent contains blog key than all children also belong to blog
        for id in DFS::new(site_tree) {
            match &site_tree[id].kind {
                SiteNodeKind::Page { page, .. } => {
                    if let Some(attributes) = page.attributes() {
                        println!("{attributes:?}");
                        if let Some(_) = attributes.get("blog") {
                            self.enabled_site_ids.insert(id);
                            continue;
                        }
                    }

                    if let Some(parent) = site_tree.page_parent(id) {
                        if self.enabled_site_ids.contains(&parent) {
                            self.enabled_site_ids.insert(id);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render_page<'n>(&mut self, dom: &mut DomTree, context: &RenderContext<'n>) {
        let site_tree = context.site_tree();
        let site_id = context.site_id();

        if !self.enabled_site_ids.contains(&site_id) {
            return;
        }

        // reset state
        self.has_inserted_date = false;

        // TODO make shorter
        let body = dom.get_elements_by_tag_name("body")[0];

        // add breacrumbs
        {
            let nav = dom.add_element_with_attributes(
                body,
                "nav",
                to_attributes([("class", "breadcrumbs")]),
            );

            dom.add_text(nav, "/");

            let parents = site_tree.parents(site_id);
            let parents_length = parents.len();
            for (i, p) in parents.into_iter().rev().enumerate() {
                let a = dom.add_element_with_attributes(
                    nav,
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                );
                if i != parents_length - 1 {
                    dom.add_text(nav, "/");
                }
                dom.add_text(a, site_tree[p].name.clone());
            }
            dom.add_text(nav, format!("/{}", site_tree[site_id].name));
        }
    }

    fn render_body<'n>(
        &mut self,
        dom: &mut DomTree,
        context: &RenderContext<'n>,
        parent_dom_id: usize,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> bool {
        let site_id = context.site_id();
        if !self.enabled_site_ids.contains(&site_id) {
            return false;
        }

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                // match get_date(context) {
                //     Ok(date) => {
                //         let date: DateTime<Utc> = date.into();
                //         let date = date.format("Updated on %B %d, %Y").to_string();
                //
                //         let div = dom.add_element_with_attributes(
                //             parent_dom_id,
                //             "div",
                //             to_attributes([("class", "post-updated-on")]),
                //         );
                //         dom.add_text(div, date);
                //
                //         self.has_inserted_date = true;
                //     }
                //     Err(e) => error!("failed to read date from post: {e}"),
                // }
            }
            _ => {}
        }
        return false;
    }
}

/// get the date from input and options
fn get_date(context: &RenderContext) -> Result<DateTime<Utc>, LssgError> {
    todo!()
    // match input {
    //     Input::Local { path } => {}
    //     Input::External { url } => todo!(),
    // }
}
