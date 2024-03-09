use std::{
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    rc::Rc,
};

use log::warn;

use super::{DefaultModule, RenderContext, RendererModule};
use crate::{
    dom::{DomNode, DomNodeKind, DomTree, WeakDomNode},
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

    /// Render using other modules
    pub fn render_down(
        &mut self,
        current_module: &dyn RendererModule,
        dom: &mut DomTree,
        context: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &Vec<Token>,
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if current_module.id() == module.id() {
                    continue;
                }
                if let Some(p) = module.render_body(dom, context, parent.clone(), &token, self) {
                    parent = p;
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
        parent
    }

    pub fn render(
        &mut self,
        dom: &mut DomTree,
        context: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &Vec<Token>,
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if let Some(p) = module.render_body(dom, context, parent.clone(), &token, self) {
                    parent = p;
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
        parent
    }

    /// consume self and return a parsed domtree
    pub fn start_render(mut self, dom: &mut DomTree, context: &RenderContext) {
        let body = dom.body();
        let tokens = context.page.tokens();
        self.render(dom, context, body, tokens);
    }
}
