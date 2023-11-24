use crate::sitetree::{Page, SiteTree};

pub struct RenderContext<'n> {
    pub site_tree: &'n SiteTree,
    pub site_id: usize,
    pub page: &'n Page,
}

impl<'n> RenderContext<'n> {
    pub fn new(site_tree: &'n SiteTree, site_id: usize, page: &'n Page) -> Self {
        RenderContext {
            site_tree,
            site_id,
            page,
        }
    }

    pub fn site_tree(&self) -> &SiteTree {
        self.site_tree
    }

    pub fn site_id(&self) -> usize {
        self.site_id
    }

    pub fn page(&self) -> &Page {
        &self.page
    }
}
