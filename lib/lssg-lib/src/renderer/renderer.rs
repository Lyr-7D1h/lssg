use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    domtree::DomTree,
    lmarkdown::Token,
    sitetree::{SiteNodeKind, SiteTree},
    LssgError,
};

use super::modules::{RendererModule, RendererModuleContext};
use super::RenderQueue;

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

    /// Will run site_init on all modules, will remove modules if it fails
    pub fn site_init(&mut self, site_tree: &mut SiteTree) {
        let failed: Vec<usize> = (&mut self.modules)
            .into_iter()
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

    /// Transform site id into a html page
    pub fn render(&mut self, site_tree: &SiteTree, site_id: usize) -> Result<String, LssgError> {
        // get the site node
        let site_node = site_tree.get(site_id)?;
        let (tokens, ..) = match &site_node.kind {
            SiteNodeKind::Page { tokens, input, .. } => (tokens, input),
            _ => return Err(LssgError::render("Invalid node type given")),
        };

        let mut tree = DomTree::new();

        let context = RendererModuleContext {
            site_tree,
            site_id,
            tokens,
        };

        // initialize modules
        for module in &mut self.modules {
            module.render_page(&mut tree, &context);
        }

        // create body
        let body = tree.get_elements_by_tag_name("body")[0];
        let mut queue = RenderQueue::from_tokens(context.tokens.clone(), body);
        'l: while let Some((token, parent_id)) = queue.pop_front() {
            for module in &mut self.modules {
                if module.render_body(&mut tree, &context, &mut queue, parent_id, &token) {
                    continue 'l;
                }
            }
            warn!("{token:?} not renderered");
        }

        // println!("{tree}");
        return Ok(tree.to_html_string());
    }
}