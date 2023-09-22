use std::{collections::HashMap, path::PathBuf};

use crate::{
    domtree::{to_attributes, DomTree},
    lssg_error::LssgError,
    parser::lexer::{self, Token},
    sitetree::{SiteNode, SiteNodeKind, SiteTree},
};

use super::RendererModule;

struct BlogModule<'n> {
    site_tree: &'n SiteTree,
    blog_enabled_site_ids: Vec<usize>,
}

impl<'n> RendererModule<'n> for BlogModule<'n> {
    fn new(site_tree: &'n SiteTree) -> Self {
        Self {
            site_tree,
            blog_enabled_site_ids: vec![],
        }
    }

    fn body(
        &self,
        token: lexer::Token,
        tree: &mut crate::domtree::DomTree,
        parent_dom_id: usize,
    ) -> Option<String> {
        match token {
            Token::Link { text, href } => {}
            _ => {}
        }
        todo!()
    }

    fn init(
        &mut self,
        site_id: usize,
        site_node: &SiteNode,
        _site_node_input: &PathBuf,
        tokens: &Vec<Token>,
        tree: &mut DomTree,
    ) {
        let metadata = if let Some(Token::Comment { text: _, map }) = tokens.first() {
            map.clone()
        } else {
            HashMap::new()
        };

        if metadata.contains_key("blog") {
            self.blog_enabled_site_ids.push(site_id)
        } else {
            let mut has_blog_parent = false;
            for id in &self.blog_enabled_site_ids {
                if self.site_tree.is_parent(site_id, *id) {
                    has_blog_parent = true;
                    break;
                }
            }
            if has_blog_parent == false {
                return;
            }
        }

        let body = tree.get_elements_by_tag_name("body")[0];
        let nav = tree.add_element_with_attributes(
            "nav",
            to_attributes([("class", "breadcrumbs")]),
            body,
        );

        let mut parent = site_node.parent;
        while let Some(p) = parent {
            let node = self.site_tree[p].parent;
            let a = tree.add_element_with_attributes(
                "a",
                to_attributes([("href", self.site_tree.rel_path(site_id, p))]),
                nav,
            );
            tree.add_text(self.site_tree[p].name.clone(), a);
            parent = node;
        }
    }
}
