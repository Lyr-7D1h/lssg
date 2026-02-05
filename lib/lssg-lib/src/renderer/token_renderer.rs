use log::warn;

use super::{RenderContext, RendererModule};
use crate::lmarkdown::Token;
use virtual_dom::{Document, DomNode};

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
        document: &mut Document,
        ctx: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &[Token],
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            let mut modules_iter = modules.iter_mut();
            // skip all before current module
            for module in modules_iter.by_ref() {
                if module.id() == current_module.id() {
                    break;
                }
            }
            for module in modules_iter {
                if current_module.id() == module.id() {
                    continue;
                }
                if let Some(p) = module.render_token(document, ctx, parent.clone(), token, self) {
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
        document: &mut Document,
        ctx: &RenderContext<'a>,
        mut parent: DomNode,
        tokens: &[Token],
    ) -> DomNode {
        'l: for token in tokens.iter() {
            let modules = unsafe { self.modules.as_mut().unwrap() };
            for module in modules.iter_mut() {
                if let Some(p) = module.render_token(document, ctx, parent.clone(), token, self) {
                    parent = p;
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }
        parent
    }

    /// consume self and return a parsed domtree
    pub fn start_render(mut self, document: &mut Document, ctx: &RenderContext) {
        let tokens = ctx.page.tokens();
        self.render(document, ctx, document.body.clone(), tokens);
    }
}
