use std::iter;

use log::error;
use serde_extensions::Overwrite;

use crate::{
    LssgError,
    lmarkdown::Token,
    renderer::InitContext,
    sitetree::{Page, SiteId, SiteTree},
};
use virtual_dom::{Document, DomNode};

mod external_module;
pub mod model_module;
pub use external_module::*;
mod post_module;
pub use post_module::*;
mod default_module;
pub use default_module::*;
mod media_module;
pub use media_module::*;
pub mod util;

use super::{RenderContext, TokenRenderer};

/// Implement a custom RendererModule
///
/// Function order:
/// ```md
/// | Changing site tree | Look at final site tree | Page Rendering
/// init()               -> after_init()           -> render_page() -> render_body() -> after_render()
/// ```
///
#[allow(unused)]
pub trait RendererModule {
    /// Return a static identifier for this module
    fn id(&self) -> &'static str;

    /// This gets run once just after the site_tree has been created
    ///
    /// Its mostly useful for modifying the site tree (adding new pages, modifying resources, etc.)
    fn init(&mut self, ctx: InitContext) -> Result<(), LssgError> {
        Ok(())
    }

    /// Gets run after all changes to site tree has been made
    fn after_init(&mut self, site_tree: &SiteTree) -> Result<(), LssgError> {
        Ok(())
    }

    /// Modify DomTree before rendering page
    ///
    /// return Some(String) if you want to render the page yourself and ignore renderer for this page
    fn render_page<'n>(&mut self, dom: &mut Document, ctx: &RenderContext<'n>) -> Option<String> {
        None
    }

    /// Render a single token by appending to parent
    ///
    /// returns the Some(new_parent) if it rendered given token otherwise None and will continue to next render module
    fn render_token<'n>(
        &mut self,
        document: &mut Document,
        ctx: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        None
    }

    /// Gets called after body has been rendered, can be used for final changes to the dom
    fn after_render<'n>(&mut self, document: &mut Document, ctx: &RenderContext<'n>) {}

    /// Find root of `current_site_id` and apply all options from parents
    fn propegated_options<D: Overwrite + Default>(
        &self,
        current_site_id: SiteId,
        site_tree: &SiteTree,
    ) -> D
    where
        Self: Sized,
    {
        self.propegated_options_with_module_id(current_site_id, site_tree, self.id())
    }

    // TODO: remove when everything is in own module
    /// Find root of `current_site_id` and apply all options from parents
    fn propegated_options_with_module_id<D: Overwrite + Default>(
        &self,
        current_site_id: SiteId,
        site_tree: &SiteTree,
        module_id: &'static str,
    ) -> D
    where
        Self: Sized,
    {
        let mut parent_pages = vec![];
        for site_id in iter::once(current_site_id).chain(site_tree.parents(current_site_id)) {
            let Some(attributes) = site_tree.page(site_id).and_then(|p| p.attributes()) else {
                continue;
            };
            let is_root = site_id == site_tree.root()
                || attributes
                    .get("root")
                    .and_then(|v| v.as_bool())
                    .is_some_and(|v| v);
            parent_pages.push((site_id, attributes));
            if is_root {
                break;
            }
        }

        let mut options = D::default();
        while let Some((id, attributes)) = parent_pages.pop() {
            if module_id == "default" {
                if let Err(e) = options.overwrite(attributes) {
                    error!("Failed to parse options for '{module_id}' module: {e}",)
                }
                continue;
            }
            if let Some(v) = attributes.get(module_id)
                && let Err(e) = options.overwrite(v.clone())
            {
                error!("Failed to parse options for '{}' module: {e}", module_id)
            }
        }

        options
    }

    fn options<D: Overwrite + Default>(&self, page: &Page) -> Option<D>
    where
        Self: Sized,
    {
        self.options_with_module_id(page, self.id())
    }

    // TODO: remove when everything is in own module
    /// get default options overwritten with Token::Attributes
    fn options_with_module_id<D: Overwrite + Default>(
        &self,
        page: &Page,
        module_id: &'static str,
    ) -> Option<D>
    where
        Self: Sized,
    {
        // None if no attributes
        let toml = page.attributes()?;

        let mut v = D::default();

        // if default use root table
        if module_id == "default" {
            if let Err(e) = v.overwrite(toml.clone()) {
                error!("Failed to parse options for '{}' module: {e}", module_id)
            }
            return Some(v);
        }

        // module key must exist in attributes
        let table = toml.get(module_id)?;
        if let Err(e) = v.overwrite(table.clone()) {
            error!("Failed to parse options for '{}' module: {e}", module_id)
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lmarkdown::Token;
    use crate::sitetree::{Input, SiteNode, SiteNodeKind};
    use serde_extensions::Overwrite;

    #[derive(Debug, Default, PartialEq, Overwrite)]
    struct TestOptions {
        name: String,
        count: i32,
        enabled: bool,
    }

    // Mock renderer module for testing
    struct TestModule;
    impl RendererModule for TestModule {
        fn id(&self) -> &'static str {
            "test"
        }
    }

    // Helper to create a page with attributes
    fn create_page_with_attributes(attributes_toml: &str) -> Page {
        let mut page = Page::empty();
        if !attributes_toml.trim().is_empty() {
            let table: toml::Table = toml::from_str(attributes_toml).unwrap();
            let token = Token::Attributes { table };
            page.tokens_mut().insert(0, token);
        }
        page
    }

    // Helper to create a basic site tree for testing
    fn create_test_site_tree(pages: Vec<(Option<SiteId>, &str)>) -> SiteTree {
        use std::fs;

        // Create a temporary file for testing
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("lssg_test_page.md");
        fs::write(&test_file, "# Test").unwrap();

        let input = Input::from_string_single(
            &test_file.to_str().unwrap(),
            &reqwest::blocking::Client::new(),
        )
        .unwrap();

        // Create initial tree with empty root page
        let root_page = if !pages.is_empty() && pages[0].0.is_none() {
            create_page_with_attributes(pages[0].1)
        } else {
            Page::empty()
        };

        let mut tree = SiteTree::from_input(input, reqwest::blocking::Client::new()).unwrap();

        // Replace root page with the one we want using IndexMut
        if let SiteNodeKind::Page(page) = &mut tree[SiteId(0)].kind {
            *page = root_page;
        }

        // Skip first element if it was used as root
        let start_index = if !pages.is_empty() && pages[0].0.is_none() {
            1
        } else {
            0
        };

        for (idx, (parent, attributes_toml)) in pages.iter().enumerate().skip(start_index) {
            let page = create_page_with_attributes(attributes_toml);
            let name = format!("page_{}", idx);
            let parent_id = parent.unwrap_or(SiteId(0));
            let node = SiteNode::page(name, parent_id, page);
            tree.add(node);
        }

        tree
    }

    #[test]
    fn test_propegated_options_no_parents() {
        let tree = create_test_site_tree(vec![(
            None,
            r#"
                [test]
                name = "root"
                count = 42
                enabled = true
            "#,
        )]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(0), &tree);

        assert_eq!(options.name, "root");
        assert_eq!(options.count, 42);
        assert_eq!(options.enabled, true);
    }

    #[test]
    fn test_propegated_options_single_parent() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                [test]
                name = "parent"
                count = 10
                enabled = false
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                [test]
                count = 20
            "#,
            ),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(1), &tree);

        // name should be inherited from parent
        assert_eq!(options.name, "parent");
        // count should be overwritten by child
        assert_eq!(options.count, 20);
        // enabled should be inherited from parent
        assert_eq!(options.enabled, false);
    }

    #[test]
    fn test_propegated_options_multiple_levels() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                [test]
                name = "grandparent"
                count = 1
                enabled = true
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                [test]
                count = 2
            "#,
            ),
            (
                Some(SiteId(1)),
                r#"
                [test]
                enabled = false
            "#,
            ),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(2), &tree);

        // name inherited from grandparent
        assert_eq!(options.name, "grandparent");
        // count inherited from parent
        assert_eq!(options.count, 2);
        // enabled overwritten by child
        assert_eq!(options.enabled, false);
    }

    #[test]
    fn test_propegated_options_stops_at_root_attribute() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                [test]
                name = "root"
                count = 100
                enabled = true
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                root = true
                [test]
                name = "middle"
                count = 50
            "#,
            ),
            (
                Some(SiteId(1)),
                r#"
                [test]
                enabled = false
            "#,
            ),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(2), &tree);

        // Should only inherit from middle node (which has root=true) and child
        // Should NOT inherit from the actual root
        assert_eq!(options.name, "middle");
        assert_eq!(options.count, 50);
        assert_eq!(options.enabled, false);
    }

    #[test]
    fn test_propegated_options_no_module_key() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                name = "root"
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                count = 5
            "#,
            ),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(1), &tree);

        // Should get default values when module key doesn't exist
        assert_eq!(options.name, "");
        assert_eq!(options.count, 0);
        assert_eq!(options.enabled, false);
    }

    #[test]
    fn test_propegated_options_with_default_module() {
        struct DefaultModule;
        impl RendererModule for DefaultModule {
            fn id(&self) -> &'static str {
                "default"
            }
        }

        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                name = "root"
                count = 15
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                enabled = true
            "#,
            ),
        ]);

        let module = DefaultModule;
        let options: TestOptions = module.propegated_options(SiteId(1), &tree);

        // For 'default' module, should use root table directly
        assert_eq!(options.name, "root");
        assert_eq!(options.count, 15);
        assert_eq!(options.enabled, true);
    }

    #[test]
    fn test_propegated_options_empty_child() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                [test]
                name = "parent"
                count = 30
                enabled = true
            "#,
            ),
            (Some(SiteId(0)), ""),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(1), &tree);

        // Should inherit all from parent when child has no attributes
        assert_eq!(options.name, "parent");
        assert_eq!(options.count, 30);
        assert_eq!(options.enabled, true);
    }

    #[test]
    fn test_propegated_options_partial_override() {
        let tree = create_test_site_tree(vec![
            (
                None,
                r#"
                [test]
                name = "base"
                count = 100
                enabled = false
            "#,
            ),
            (
                Some(SiteId(0)),
                r#"
                [test]
                name = "override"
            "#,
            ),
        ]);

        let module = TestModule;
        let options: TestOptions = module.propegated_options(SiteId(1), &tree);

        // Only name should be overridden
        assert_eq!(options.name, "override");
        assert_eq!(options.count, 100);
        assert_eq!(options.enabled, false);
    }
}
