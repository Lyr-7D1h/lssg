use log::error;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    sitetree::{Page, SiteTree},
    LssgError,
};
use virtual_dom::{Document, DomNode};

mod blog_module;
pub use blog_module::*;
mod default_module;
pub use default_module::*;

use super::{RenderContext, TokenRenderer};

#[allow(unused)]
pub trait RendererModule {
    /// Return a static identifier for this module
    fn id(&self) -> &'static str;

    /// This gets run once just after site_tree has been created
    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Gets run after all changes to site tree has been made
    fn after_init(&mut self, site_tree: &SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Modify DomTree before rendering page
    fn render_page<'n>(&mut self, dom: &mut Document, context: &RenderContext<'n>) {}

    /// Render a token before default token renderer returns parent id for following tokens if it parsed this token otherwise None
    fn render_body<'n>(
        &mut self,
        dom: &mut Document,
        context: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        None
    }

    /// Gets called after body has been rendered, can be used for final changes to the dom
    fn after_render<'n>(&mut self, dom: &mut Document, context: &RenderContext<'n>) {}

    /// get options by overwriting `default` with Token::Attributes
    fn options_with_default<D: Overwrite + Default>(&self, page: &Page, mut default: D) -> D
    where
        Self: Sized,
    {
        if let Some(Token::Attributes { table: toml }) = page.tokens().first() {
            // default options are defined on root of table
            if self.id() == "default" {
                if let Err(e) = default.overwrite(toml.clone()) {
                    error!("Failed to parse options for '{}' module: {e}", self.id())
                }
                return default;
            }

            if let Some(v) = toml.get(self.id()) {
                if let Err(e) = default.overwrite(v.clone()) {
                    error!("Failed to parse options for '{}' module: {e}", self.id())
                }
            }
        }
        default
    }

    /// get default options overwritten with Token::Attributes
    fn options<D: Overwrite + Default>(&self, page: &Page) -> D
    where
        Self: Sized,
    {
        let mut o = D::default();
        if let Some(Token::Attributes { table: toml }) = page.tokens().first() {
            // default options are defined on root of table
            if self.id() == "default" {
                if let Err(e) = o.overwrite(toml.clone()) {
                    error!("Failed to parse options for '{}' module: {e}", self.id())
                }
                return o;
            }

            if let Some(v) = toml.get(self.id()) {
                if let Err(e) = o.overwrite(v.clone()) {
                    error!("Failed to parse options for '{}' module: {e}", self.id())
                }
            }
        }
        o
    }
}
