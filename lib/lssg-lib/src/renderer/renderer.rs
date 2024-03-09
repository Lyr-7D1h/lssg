use std::cell::{RefCell, UnsafeCell};
use std::rc::Rc;

use log::{debug, error, info, warn};

use crate::sitetree::Input;
use crate::{
    dom::DomTree,
    sitetree::{SiteNodeKind, SiteTree},
    LssgError,
};

use super::{modules::RendererModule, DefaultModule};
use super::{RenderContext, TokenRenderer};

/// HtmlRenderer is responsible for the process of converting the site tree into the final HTML output.
/// It does this by managing a queue of tokens to be rendered and delegating the rendering process to different modules.
pub struct Renderer {
    modules: Vec<Box<dyn RendererModule>>,
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer { modules: vec![] }
    }

    pub fn add_module(&mut self, module: impl RendererModule + 'static) {
        self.modules.push(Box::new(module));
    }

    /// Will run init on all modules, will remove modules if it fails
    pub fn init(&mut self, site_tree: &mut SiteTree) {
        debug!("running init");
        let failed: Vec<usize> = self
            .modules
            .iter_mut()
            .enumerate()
            .filter_map(|(i, module)| match module.init(site_tree) {
                Ok(_) => None,
                Err(e) => {
                    error!("Failed to do site_init on {}: {e}", module.id());
                    Some(i)
                }
            })
            .collect();
        for i in failed.into_iter().rev() {
            self.modules.remove(i);
        }
    }

    /// Will run after_init on all modules, will remove modules if it fails
    pub fn after_init(&mut self, site_tree: &SiteTree) {
        debug!("running after_init");
        let failed: Vec<usize> = self
            .modules
            .iter_mut()
            .enumerate()
            .filter_map(|(i, module)| match module.after_init(site_tree) {
                Ok(_) => None,
                Err(e) => {
                    error!("Failed to do site_init on {}: {e}", module.id());
                    Some(i)
                }
            })
            .collect();
        for i in failed.into_iter().rev() {
            self.modules.remove(i);
        }
    }

    /// Transform site id into a html page
    pub fn render(&mut self, site_tree: &SiteTree, site_id: usize) -> Result<String, LssgError> {
        // get the site node
        let site_node = site_tree.get(site_id)?;
        let page = match &site_node.kind {
            SiteNodeKind::Page(page) => page,
            _ => return Err(LssgError::render("Invalid node type given")),
        };

        let mut dom = DomTree::new();

        let context = RenderContext {
            input: site_tree.get_input(site_id),
            site_tree,
            site_id,
            page,
        };

        // initialize modules
        for module in &mut self.modules {
            debug!("running render_page on {}", module.id());
            module.render_page(&mut dom, &context);
        }

        debug!("running render_body on modules");
        let tr = TokenRenderer::new(&mut self.modules);
        tr.start_render(&mut dom, &context);

        for module in &mut self.modules {
            debug!("running after_render on {}", module.id());
            module.after_render(&mut dom, &context);
        }

        // sanitize html
        dom.sanitize();

        // println!("{dom}");
        // println!("{dom:?}");
        // println!("{:?}", tree.get_mut(9));
        // println!("{page:#?}");
        Ok(dom.to_string())
    }
}
