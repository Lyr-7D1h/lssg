use log::error;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    sitetree::{Page, SiteTree},
    LssgError,
};
use virtual_dom::{Document, DomNode};

mod external_module;
pub use external_module::*;
mod blog_module;
pub use blog_module::*;
mod default_module;
pub use default_module::*;
mod media_module;
pub use media_module::*;
pub mod util;

use super::{RenderContext, TokenRenderer};

/// Implement a custom RendererModule
#[allow(unused)]
pub trait RendererModule {
    /// Return a static identifier for this module
    fn id(&self) -> &'static str;

    /// This gets run once just after the site_tree has been created
    ///
    /// Its mostly useful for modifying the site tree (adding new pages, modifying resources, etc.)
    fn init(&mut self, site_tree: &mut SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Gets run after all changes to site tree has been made
    fn after_init(&mut self, site_tree: &SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Modify DomTree before rendering page
    ///
    /// return Some(String) if you want to render the page yourself and ignore renderer for this page
    fn render_page<'n>(
        &mut self,
        dom: &mut Document,
        context: &RenderContext<'n>,
    ) -> Option<String> {
        None
    }

    /// Render a single token by appending to parent
    ///
    /// returns the Some(new_parent) if it rendered given token otherwise None and will continue to next render module
    fn render_body<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        None
    }

    /// Gets called after body has been rendered, can be used for final changes to the dom
    fn after_render<'n>(&mut self, document: &mut Document, context: &RenderContext<'n>) {}

    /// get options by overwriting provided `default` with Token::Attributes
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
