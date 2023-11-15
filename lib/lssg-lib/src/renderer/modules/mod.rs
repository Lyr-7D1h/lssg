use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    domtree::DomTree,
    lmarkdown::Token,
    sitetree::{SiteNodeKind, SiteTree},
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

    fn options_with_default<D: Overwrite + Default>(&self, tokens: &Vec<Token>, mut default: D) -> D
    where
        Self: Sized,
    {
        if let Some(Token::Attributes { toml }) = tokens.first() {
            if let Some(v) = toml.get(self.id()) {
                match default.overwrite(v.clone()) {
                    Ok(d) => d,
                    Err(e) => error!("Failed to parse options for '{}' module: {e}", self.id()),
                }
            }
        }
        default
    }
    fn options<D: Overwrite + Default>(&self, tokens: &Vec<Token>) -> D
    where
        Self: Sized,
    {
        let mut o = D::default();
        if let Some(Token::Attributes { toml }) = tokens.first() {
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
    pub tokens: &'n Vec<Token>,
}

impl<'n> RendererModuleContext<'n> {
    pub fn options<D: Overwrite + Default>(&self, module: &impl RendererModule) -> D {
        let mut o = D::default();
        if let Some(Token::Attributes { toml }) = self.tokens.first() {
            if let Some(v) = toml.get(module.id()) {
                if let Ok(d) = o.overwrite(v.clone()) {
                    d
                }
            }
        }
        o
    }
}
