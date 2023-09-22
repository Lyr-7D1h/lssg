use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use log::error;

use crate::{
    domtree::{to_attributes, DomTree},
    parser::lexer::{self, Token},
    sitetree::{SiteNode, SiteNodeKind, SiteTree},
};

use super::{RendererModule, RendererModuleProperties};

pub struct BlogModule {
    blog_enabled_site_ids: Vec<usize>,
    has_inserted_date: bool,
}

impl<'n> BlogModule {
    pub fn new() -> Self {
        Self {
            blog_enabled_site_ids: vec![],
            has_inserted_date: false,
        }
    }
}

impl RendererModule for BlogModule {
    fn init(
        &mut self,
        tree: &mut DomTree,
        site_tree: &SiteTree,
        site_id: usize,
        tokens: &Vec<Token>,
    ) {
        // reset state
        self.has_inserted_date = false;

        let site_node = &site_tree[site_id];

        let metadata = if let Some(Token::Comment { text: _, map }) = tokens.first() {
            map.clone()
        } else {
            HashMap::new()
        };

        // Check if blog is enabled for page or child of blog enabled page
        if metadata.contains_key("blog") {
            self.blog_enabled_site_ids.push(site_id)
        } else {
            let mut has_blog_parent = false;
            for id in &self.blog_enabled_site_ids {
                if site_tree.is_parent(site_id, *id) {
                    has_blog_parent = true;
                    break;
                }
            }
            if has_blog_parent == false {
                return;
            }
        }

        let body = tree.get_elements_by_tag_name("body")[0];
        // add breacrumbs
        {
            let nav = tree.add_element_with_attributes(
                "nav",
                to_attributes([("class", "breadcrumbs")]),
                body,
            );

            let mut parent = site_node.parent;
            while let Some(p) = parent {
                let node = site_tree[p].parent;
                let a = tree.add_element_with_attributes(
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                    nav,
                );
                tree.add_text(site_tree[p].name.clone(), a);
                parent = node;
            }
        }
    }

    fn body(
        &mut self,
        tree: &mut DomTree,
        site_tree: &SiteTree,
        site_id: usize,
        tokens: &Vec<Token>,
        token: Token,
        parent_dom_id: usize,
    ) -> bool {
        match token {
            Token::Heading { depth: 1, .. } if !self.has_inserted_date => {
                if let SiteNodeKind::Page { input, .. } = &site_tree[site_id].kind {
                    match input.metadata() {
                        Ok(m) => match m.modified() {
                            Ok(date) => {
                                let date: DateTime<Utc> = date.into();
                                let date = date.format("Updated on %B %d, %Y").to_string();
                                let div = tree.add_element_with_attributes(
                                    "div",
                                    to_attributes([("class", "post-updated-on")]),
                                    parent_dom_id,
                                );
                                tree.add_text(date, div);
                                return true;
                            }
                            Err(e) => {
                                error!("failed to read modified date from input metadata: {e}")
                            }
                        },
                        Err(e) => error!("failed to read input metadata: {e}"),
                    }
                }
            }
            _ => {}
        }
        todo!()
    }
}
