use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::{
    renderer::{InitContext, modules::sitemap_module::sitemap::Sitemap},
    sitetree::{Resource, SiteNode},
};

use super::RendererModule;

mod sitemap;

/// Options for the sitemap module, read from page attributes under `[sitemap]`
#[derive(Overwrite, Clone, Debug, Deserialize)]
pub(super) struct SitemapOptions {
    /// Host URL (e.g. "https://example.com").
    /// If not set, the sitemap will only contain relative paths.
    #[serde(default)]
    pub host: Option<String>,
}
impl Default for SitemapOptions {
    fn default() -> Self {
        Self { host: None }
    }
}

/// A module that generates a `sitemap.xml` for the site.
///
/// On `init`, it scans all pages in the site tree and builds a
/// [Sitemap Protocol](https://www.sitemaps.org/protocol.html) XML file,
/// then adds it as a static resource under the root node.
#[derive(Default)]
pub struct SitemapModule;

impl RendererModule for SitemapModule {
    fn id(&self) -> &'static str {
        "sitemap"
    }

    fn init(&mut self, InitContext { site_tree, .. }: InitContext) -> Result<(), crate::LssgError> {
        let root = site_tree.root();

        // Read options propagated from the root page, falling back to defaults
        let options: SitemapOptions = self.propegated_options(root, site_tree);

        // Collect all pages and build the sitemap
        let sitemap = Sitemap::build(site_tree, &options);
        let content = sitemap.to_string();

        let sitemap_resource =
            SiteNode::resource("sitemap.xml", root, Resource::new_static(content));
        site_tree.add(sitemap_resource);

        Ok(())
    }
}
