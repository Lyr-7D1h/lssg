use crate::{
    html::{to_attributes, DomTree},
    lmarkdown::Token,
    renderer::RenderContext,
    sitetree::SiteNodeKind,
};

use super::{RendererModule, TokenRenderer};

pub struct BlogModule {
    post_enabled_site_ids: Vec<usize>,
    blog_root_site_ids: Vec<usize>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            post_enabled_site_ids: vec![],
            blog_root_site_ids: vec![],
            has_inserted_date: false,
        }
    }
}

impl RendererModule for BlogModule {
    fn id(&self) -> &'static str {
        return "blog";
    }

    fn render_page(&mut self, tree: &mut DomTree, context: &super::RenderContext) {
        let RenderContext {
            site_tree, site_id, ..
        } = context;
        let site_id = *site_id;
        // reset state
        self.has_inserted_date = false;

        // Check if blog is enabled for page or child of blog enabled page
        // if metadata.contains_key("blog") {
        //     self.blog_root_site_ids.push(site_id);
        // } else {
        let mut has_blog_parent = false;
        for id in &self.blog_root_site_ids {
            if site_tree.is_parent(site_id, *id) {
                has_blog_parent = true;
                break;
            }
        }
        if has_blog_parent == false {
            return;
        }
        self.post_enabled_site_ids.push(site_id);
        // }

        let body = tree.get_elements_by_tag_name("body")[0];

        // add breacrumbs
        {
            let nav = tree.add_element_with_attributes(
                body,
                "nav",
                to_attributes([("class", "breadcrumbs")]),
            );

            tree.add_text(nav, "/");

            let parents = site_tree.parents(site_id);
            let parents_length = parents.len();
            for (i, p) in parents.into_iter().rev().enumerate() {
                let a = tree.add_element_with_attributes(
                    nav,
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                );
                if i != parents_length - 1 {
                    tree.add_text(nav, "/");
                }
                tree.add_text(a, site_tree[p].name.clone());
            }
            tree.add_text(nav, format!("/{}", site_tree[site_id].name));
        }
    }

    fn render_body<'n>(
        &mut self,
        tr: &mut TokenRenderer,
        _parent_dom_id: usize,
        token: &Token,
    ) -> bool {
        let site_id = tr.site_id();
        match token {
            Token::Heading { depth, .. }
                if *depth != 1
                    && token.is_text()
                    && !self.has_inserted_date
                    && self.post_enabled_site_ids.contains(&site_id) =>
            {
                if let SiteNodeKind::Page { .. } = &tr.site_tree()[site_id].kind {
                    // TODO get date
                    // match input.metadata() {
                    //     Ok(m) => match m.modified() {
                    //         Ok(date) => {
                    //             self.has_inserted_date = true;
                    //             let date: DateTime<Utc> = date.into();
                    //             let date = date.format("Updated on %B %d, %Y").to_string();
                    //             let div = tree.add_element_with_attributes(
                    //                 "div",
                    //                 to_attributes([("class", "post-updated-on")]),
                    //                 parent_dom_id,
                    //             );
                    //             tree.add_text(date, div);
                    //         }
                    //         Err(e) => {
                    //             error!("failed to read modified date from input metadata: {e}")
                    //         }
                    //     },
                    //     Err(e) => error!("failed to read input metadata: {e}"),
                    // }
                }
            }
            _ => {}
        }
        return false;
    }
}
