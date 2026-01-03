use std::collections::HashMap;

use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    renderer::{
        BlogModule, BlogPostOptions, BlogRootOptions, RendererModule,
        modules::blog_module::blog_post_dates::BlogPostDates,
    },
    sitetree::{SiteId, SiteNodeKind, SiteTree},
    tree::{Ancestors, Dfs},
};

#[derive(Debug, Clone)]
/// Describes the content of a blog post
pub(super) struct Contents {
    pub title: Option<String>,
}
impl Contents {
    /// Extract title and description from page tokens
    fn from_page(page: &crate::sitetree::Page) -> Self {
        let title = page.tokens().iter().find_map(|t| {
            if let crate::lmarkdown::Token::Heading { text, depth, .. } = t
                && *depth == 1 {
                    return Some(text.clone());
                }

            None
        });

        Self { title }
    }
}

#[derive(Debug, Clone)]
pub(super) struct PostPage {
    pub post_options: BlogPostOptions,
    /// Relevant dates given by metadata
    pub dates: BlogPostDates,
    /// Contents from tokens
    pub contents: Contents,
}

#[derive(Debug)]
/// Represent a page active in the blog module
pub(super) struct RootPage {
    /// Global blog settings applied to all children
    pub options: BlogRootOptions,
    pub posts: HashMap<SiteId, PostPage>,
}

impl BlogModule {
    pub(super) fn collect_roots(&self, site_tree: &mut SiteTree) -> HashMap<SiteId, RootPage> {
        let mut roots = HashMap::new();

        // let pages = Dfs::new(site_tree).filter(|id| site_tree[*id].kind.is_page());
        // if contains module id it is a blog post
        for site_id in Dfs::new(site_tree) {
            if let SiteNodeKind::Page(page) = &site_tree[site_id].kind {
                let Some(table) = page.attributes().and_then(|a| a.get(self.id()).cloned())
                else {
                    continue;
                };

                let post_page = |root_options: &BlogRootOptions| {
                    table.get("post").map(|v| {
                        let mut post_options = BlogPostOptions::default();
                        if let Err(e) = post_options.overwrite(v.clone()) {
                            error!("Failed to parse options for '{}' module: {e}", self.id())
                        }

                        let input = if root_options.use_fs_dates {
                            site_tree.get_input(site_id).cloned()
                        } else {
                            None
                        };
                        let dates = BlogPostDates::from_post_options(&post_options, &input)
                            .inspect_err(|e| warn!("Failed to parse dates: {e}"))
                            .ok()
                            .unwrap_or_default();

                        PostPage {
                            post_options,
                            dates,
                            contents: Contents::from_page(page),
                        }
                    })
                };

                // if this is root make a new root page
                if let Some(v) = table.get("root") {
                    let mut options = BlogRootOptions::default();
                    if let Err(e) = options.overwrite(v.clone()) {
                        error!("Failed to parse options for '{}' module: {e}", self.id())
                    }
                    let mut posts = HashMap::new();
                    if let Some(page) = post_page(&options) {
                        posts.insert(site_id, page);
                    }
                    roots.insert(site_id, RootPage { posts, options });
                    continue;
                };

                let Some(root) =
                    Ancestors::new(site_tree, site_id).find(|id| roots.contains_key(id))
                else {
                    let options = BlogRootOptions::default();
                    if let Some(page) = post_page(&options) {
                        // if not root found make a new root if this is a post page
                        let mut posts = HashMap::new();
                        posts.insert(site_id, page);
                        roots.insert(
                            site_id,
                            RootPage {
                                options,
                                posts: HashMap::new(),
                            },
                        );
                    }
                    continue;
                };

                if let Some(root) = roots.get_mut(&root)
                    && let Some(page) = post_page(&root.options)
                {
                    // add post page to root
                    root.posts.insert(site_id, page);
                }
            }
        }

        roots
    }
}
