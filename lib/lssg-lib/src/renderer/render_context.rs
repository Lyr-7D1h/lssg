use crate::sitetree::{Input, Page, SiteTree};

#[derive(Clone)]
pub struct RenderContext<'n> {
    pub input: &'n Input,
    pub site_tree: &'n SiteTree,
    pub site_id: usize,
    pub page: &'n Page,
}

impl<'n> RenderContext<'n> {
    pub fn input(&self) -> &Input {
        self.input
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
