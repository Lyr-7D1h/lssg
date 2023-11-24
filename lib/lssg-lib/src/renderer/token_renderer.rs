use std::{
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
};

use log::warn;

use super::{DefaultModule, RenderContext, RendererModule};
use crate::{
    html::{DomNodeKind, DomTree},
    lmarkdown::Token,
    sitetree::SiteTree,
};

/// Combination of RenderContext and DomTree and recursive render behavior
/// Populates given DomTree
pub struct TokenRenderer<'a> {
    tree: DomTree,
    modules: *mut Vec<Box<dyn RendererModule>>,
    context: RenderContext<'a>,
}

impl<'a> TokenRenderer<'a> {
    pub fn new(
        tree: DomTree,
        modules: &'a mut Vec<Box<dyn RendererModule>>,
        context: RenderContext<'a>,
    ) -> TokenRenderer<'a> {
        // turn into pointer to allow for recursive call backs in render()
        let modules: *mut Vec<Box<dyn RendererModule>> = modules;
        TokenRenderer {
            tree,
            modules,
            context,
        }
    }

    pub fn site_tree(&self) -> &SiteTree {
        self.context.site_tree()
    }

    pub fn site_id(&self) -> usize {
        self.context.site_id()
    }

    /// Returns the dom tree
    pub fn dom_tree(&mut self) -> &mut DomTree {
        &mut self.tree
    }
    // DomTree functions
    pub fn add(&mut self, parent_id: usize, kind: DomNodeKind) -> usize {
        self.tree.add(parent_id, kind)
    }
    /// Add a node to the tree return the id (index) of the node
    pub fn add_element(&mut self, parent_id: usize, tag: impl Into<String>) -> usize {
        self.tree.add_element(parent_id, tag)
    }
    pub fn add_element_with_attributes(
        &mut self,
        parent_id: usize,
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> usize {
        self.tree
            .add_element_with_attributes(parent_id, tag, attributes)
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add_text(&mut self, parent_id: usize, text: impl Into<String>) -> usize {
        self.tree.add_text(parent_id, text)
    }

    pub fn render(&mut self, parent_id: usize, tokens: &Vec<Token>) {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if module.render_body(self, parent_id, &token) {
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
    }

    /// consume self and return a parsed domtree
    pub fn populate(mut self) -> DomTree {
        let body = self.tree.get_elements_by_tag_name("body")[0];
        let tokens = self.context.page.tokens();
        self.render(body, tokens);
        self.tree
    }
}
