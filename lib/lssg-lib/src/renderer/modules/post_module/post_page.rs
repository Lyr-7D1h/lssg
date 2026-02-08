use std::collections::HashMap;

use serde_extensions::Overwrite;

use crate::{
    renderer::{PostModule, RendererModule, modules::post_module::post_dates::PostDates},
    sitetree::{SiteId, SiteTree},
};

/// [post_config]
#[derive(Overwrite, Default)]
struct PostConfigOptions {
    /// Use dates from file system to create updated on and modified on tags
    /// by default false
    ///
    /// **inherited**
    use_fs_dates: bool,
}

/// [post]
#[derive(Overwrite)]
pub struct PostOptions {
    /// Use post rendering for this page, if false it will still index this page
    render: bool,
    /// When has an article been changed (any iso date string or %Y-%m-%d)
    modified_on: Option<String>,
    created_on: Option<String>,
    tags: Option<Vec<String>>,
    summary: Option<String>,
}
impl Default for PostOptions {
    fn default() -> Self {
        Self {
            render: true,
            modified_on: None,
            created_on: None,
            tags: None,
            summary: None,
        }
    }
}

#[derive(Debug)]
pub struct PostPageOptions {
    pub use_fs_dates: bool,
    pub render: bool,
    pub modified_on: Option<String>,
    pub created_on: Option<String>,
    pub tags: Option<Vec<String>>,
    pub summary: Option<String>,
}

#[derive(Debug)]
/// Describes the content of a post post
pub(super) struct Contents {
    pub title: Option<String>,
}
impl Contents {
    /// Extract title and description from page tokens
    fn from_page(page: &crate::sitetree::Page) -> Self {
        let title = page.tokens().iter().find_map(|t| {
            if let crate::lmarkdown::Token::Heading { text, depth, .. } = t
                && *depth == 1
            {
                return Some(text.clone());
            }

            None
        });

        Self { title }
    }
}

#[derive(Debug)]
pub(super) struct PostPage {
    pub options: PostPageOptions,
    /// Relevant dates given by metadata
    pub dates: PostDates,
    /// Contents from tokens
    pub contents: Contents,
}

impl PostModule {
    pub(super) fn collect_post_pages(&self, site_tree: &mut SiteTree) -> HashMap<SiteId, PostPage> {
        let mut posts = HashMap::new();

        // if contains module id it is a post post
        for (site_id, page) in site_tree.pages() {
            let post_options = {
                let PostConfigOptions { use_fs_dates } =
                    self.propegated_options_with_module_id(site_id, site_tree, "post_config");
                let Some(PostOptions {
                    render,
                    modified_on,
                    created_on,
                    tags,
                    summary,
                }) = self.options(page)
                else {
                    continue;
                };
                PostPageOptions {
                    use_fs_dates,
                    render,
                    modified_on,
                    created_on,
                    tags,
                    summary,
                }
            };

            let dates = {
                let input = if post_options.use_fs_dates {
                    page.input()
                } else {
                    None
                };
                PostDates::from_post_options(&post_options, input)
                    .inspect_err(|e| log::warn!("Failed to parse dates: {e}"))
                    .unwrap_or_default()
            };

            let contents = Contents::from_page(page);

            posts.insert(
                site_id,
                PostPage {
                    options: post_options,
                    dates,
                    contents,
                },
            );
        }

        posts
    }
}
