use std::{
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
};

use log::warn;

use super::{DefaultModule, RenderContext, RendererModule};
use crate::{
    html::{DomId, DomNode, DomNodeKind, DomTree},
    lmarkdown::Token,
    sitetree::{Page, SiteTree},
};

/// used for recursively rendering
pub struct TokenRenderer {
    modules: *mut Vec<Box<dyn RendererModule>>,
}

impl<'a> TokenRenderer {
    pub fn new(modules: &'a mut Vec<Box<dyn RendererModule>>) -> TokenRenderer {
        // turn into pointer to allow for recursive call backs in render()
        let modules: *mut Vec<Box<dyn RendererModule>> = modules;
        TokenRenderer { modules }
    }

    pub fn render(
        &mut self,
        dom: &mut DomTree,
        context: &RenderContext<'a>,
        parent_id: DomId,
        tokens: &Vec<Token>,
    ) {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if module.render_body(dom, context, parent_id, &token, self) {
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
    }

    /// consume self and return a parsed domtree
    pub fn start_render(mut self, dom: &mut DomTree, context: &RenderContext) {
        let body = dom.get_elements_by_tag_name("body")[0];
        let tokens = context.page.tokens();
        self.render(dom, context, body, tokens);
    }
}
