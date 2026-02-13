use log::{debug, error};

use crate::{
    LssgError,
    sitetree::{SiteId, SiteNodeKind, SiteTree},
};
use virtual_dom::Document;

use super::modules::RendererModule;
use super::{InitContext, RenderContext, TokenRenderer};

/// HtmlRenderer is responsible for the process of converting the site tree into the final HTML output.
/// It does this by managing a queue of tokens to be rendered and delegating the rendering process to different modules.
#[derive(Default)]
pub struct Renderer {
    modules: Vec<Box<dyn RendererModule>>,
}

impl Renderer {
    pub fn add_module(&mut self, module: impl RendererModule + 'static) {
        self.modules.push(Box::new(module));
    }

    /// Will run init on all modules, will remove modules if it fails
    pub fn init(&mut self, site_tree: &mut SiteTree, http_client: &reqwest::blocking::Client) {
        debug!("running init");
        let failed: Vec<usize> = self
            .modules
            .iter_mut()
            .enumerate()
            .filter_map(|(i, module)| {
                match module.init(InitContext {
                    http_client,
                    site_tree,
                }) {
                    Ok(_) => None,
                    Err(e) => {
                        error!("Failed to initialize for module '{}': {e}", module.id());
                        Some(i)
                    }
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
                    error!(
                        "Failed to run after initialize for module '{}': {e}",
                        module.id()
                    );
                    Some(i)
                }
            })
            .collect();
        for i in failed.into_iter().rev() {
            self.modules.remove(i);
        }
    }

    /// Transform site id into a html page
    pub fn render(
        &mut self,
        site_tree: &SiteTree,
        site_id: SiteId,
        http_client: &reqwest::blocking::Client,
    ) -> Result<String, LssgError> {
        // get the site node
        let site_node = site_tree
            .get(site_id)
            .ok_or(LssgError::render(format!("Failed to find {site_id}")))?;
        let page = match &site_node.kind {
            SiteNodeKind::Page(page) => page,
            _ => return Err(LssgError::render("Invalid node type given")),
        };

        let mut dom = Document::new();

        let context = RenderContext {
            http_client,
            input: site_tree.get_input(site_id),
            site_tree,
            site_id,
            page,
        };

        // initialize modules
        for module in &mut self.modules {
            debug!("running render_page on {}", module.id());
            if let Some(page) = module.render_page(&mut dom, &context) {
                return Ok(page);
            }
        }

        debug!("running render_body on modules");
        let token_renderer = TokenRenderer::new(&mut self.modules);
        token_renderer.start_render(&mut dom, &context);

        for module in &mut self.modules {
            debug!("running after_render on {}", module.id());
            module.after_render(&mut dom, &context);
        }

        // sanitize html
        dom.sanitize();

        Ok(dom.to_string())
    }
}
