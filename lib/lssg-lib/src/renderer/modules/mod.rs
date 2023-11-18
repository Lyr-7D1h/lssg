use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    domtree::DomTree,
    lmarkdown::Token,
    sitetree::{Page, SiteNodeKind, SiteTree},
    LssgError,
};

mod blog_module;
pub use blog_module::*;
mod default_module;
pub use default_module::*;

use super::RenderQueue;

pub trait RendererModule {
    /// Return a static id
    fn id(&self) -> &'static str;

    /// This gets run once just after site_tree has been created
    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Gets run after all changes to site tree has been made
    fn after_init(&mut self, site_tree: &SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Modify DomTree on init
    fn render_page<'n>(&mut self, tree: &mut DomTree, context: &RendererModuleContext<'n>) {}

    /// Render a token before default token renderer returns true if it parsed this token otherwise false
    fn render_body<'n>(
        &mut self,
        dom_tree: &mut DomTree,
        context: &RendererModuleContext<'n>,
        render_queue: &mut RenderQueue,
        parent_dom_id: usize,
        token: &Token,
    ) -> bool {
        false
    }

    fn options_with_default<D: Overwrite + Default>(&self, page: &Page, mut default: D) -> D
    where
        Self: Sized,
    {
        if let Some(Token::Attributes { toml }) = page.tokens().first() {
            if let Some(v) = toml.get(self.id()) {
                match default.overwrite(v.clone()) {
                    Ok(d) => d,
                    Err(e) => error!("Failed to parse options for '{}' module: {e}", self.id()),
                }
            }
        }
        default
    }

    fn options<D: Overwrite + Default>(&self, page: &Page) -> D
    where
        Self: Sized,
    {
        let mut o = D::default();
        if let Some(Token::Attributes { toml }) = page.tokens().first() {
            if let Some(v) = toml.get(self.id()) {
                match o.overwrite(v.clone()) {
                    Ok(d) => d,
                    Err(e) => error!("Failed to parse options for '{}' module: {e}", self.id()),
                }
            }
        }
        o
    }
}

pub struct RendererModuleContext<'n> {
    pub site_tree: &'n SiteTree,
    pub site_id: usize,
    pub page: &'n Page,
}
