use crate::sitetree::{Input, Page, SiteTree};

#[derive(Clone)]
pub struct RenderContext<'n> {
    pub site_tree: &'n SiteTree,
    pub site_id: usize,
    pub page: &'n Page,
    pub input: Option<&'n Input>,
}

