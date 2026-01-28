use std::collections::HashMap;

use proc_virtual_dom::dom;
use rss::RssOptions;
use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    renderer::{
        RenderContext,
        modules::blog_module::{
            collect_roots::{PostPage, RootPage},
            constants::BLOG_STYLESHEET,
        },
    },
    sitetree::{Relation, SiteId, SiteNode, Stylesheet},
};
use virtual_dom::{Document, DomNode};

use super::{RendererModule, TokenRenderer};

mod blog_post_dates;
mod collect_roots;
mod constants;
mod rss;

#[derive(Overwrite, Clone, Debug, Deserialize, Default)]
pub struct BlogRootOptions {
    #[serde(default)]
    rss: RssOptions,
    /// Use dates from file system to create updated on and modified on tags
    /// by default false
    #[serde(default)]
    use_fs_dates: bool,
}

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub struct BlogPostOptions {
    /// Use blog rendering for this page, if false it will still index this page
    render: bool,
    /// When has an article been changed (any iso date string or %Y-%m-%d)
    modified_on: Option<String>,
    created_on: Option<String>,
    tags: Option<Vec<String>>,
    summary: Option<String>,
}
impl Default for BlogPostOptions {
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

#[derive(Default)]
pub struct BlogModule {
    roots: HashMap<SiteId, RootPage>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    /// Get blog page for render
    fn post_page(&self, site_id: SiteId) -> Option<&PostPage> {
        let page = self
            .roots
            .values()
            .find_map(|root| root.posts.get(&site_id))?;
        if !page.post_options.render {
            return None;
        }
        Some(page)
    }
}

impl RendererModule for BlogModule {
    fn id(&self) -> &'static str {
        "blog"
    }

    fn init(
        &mut self,
        site_tree: &mut crate::sitetree::SiteTree,
    ) -> Result<(), crate::lssg_error::LssgError> {
        let roots = self.collect_roots(site_tree);

        let default_stylesheet = site_tree.add(SiteNode::stylesheet(
            "blog.css",
            site_tree.root(),
            Stylesheet::from_readable(BLOG_STYLESHEET)?,
        ));

        for (root_id, root) in roots.iter() {
            for page_id in root.posts.keys() {
                site_tree.add_link(*page_id, default_stylesheet, Relation::External);
            }

            // Generate RSS feed if enabled
            if root.options.rss.enabled {
                let rss_feed = rss::RssFeed::from_root(*root_id, root, site_tree);
                let rss_content = rss_feed.to_string();

                let rss_resource = crate::sitetree::Resource::new_static(rss_content);
                let rss_filename = root
                    .options
                    .rss
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("feed.xml");

                site_tree.add(SiteNode::resource(rss_filename, *root_id, rss_resource));
            }
        }

        self.roots = roots;

        Ok(())
    }

    fn render_page<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
    ) -> Option<String> {
        let site_id = context.site_id;

        // if not a blog page
        let blog_page = self.post_page(site_id)?;

        // add article meta data
        if let Some(date) = &blog_page.dates.modified_on {
            let date = date.to_rfc3339();
            document
                .head
                .append_child(dom!(<meta property="article:modified_time" content="{date}"/>));
        }
        if let Some(date) = &blog_page.dates.created_on {
            let date = date.to_rfc3339();
            document.head.append_child(dom!(
                <meta property="article:published_time" content="{date}"/>
            ));
        }

        // reset state
        self.has_inserted_date = false;

        None
    }

    fn render_body<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        let site_id = context.site_id;

        // if not a blog page
        let blog_page = self.post_page(site_id).cloned()?;

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                self.has_inserted_date = true;
                // render heading
                tr.render_down(
                    self,
                    document,
                    context,
                    parent.clone(),
                    std::slice::from_ref(token),
                );
                if let Some(date) = blog_page.dates.to_pretty_string() {
                    parent.append_child(dom!(<p class="blog__date">{date}</p>));
                }
                return Some(parent);
            }
            Token::Link {
                tokens: text, href, ..
            } => {
                if text.is_empty() {
                    return Some(parent);
                }

                // add icon if external link
                if is_href_external(href) {
                    tr.render_down(
                        self,
                        document,
                        context,
                        parent.clone(),
                        std::slice::from_ref(token),
                    );
                    parent.append_child(dom!(<svg width="1em" height="1em" viewBox="0 0 24 24" style="cursor:pointer"><g stroke-width="2.1" stroke="#666" fill="none" stroke-linecap="round" stroke-linejoin="round"><polyline points="17 13.5 17 19.5 5 19.5 5 7.5 11 7.5"></polyline><path d="M14,4.5 L20,4.5 L20,10.5 M20,4.5 L11,13.5"></path></g></svg>));
                    return Some(parent);
                }
            }
            _ => {}
        }
        None
    }
}

pub fn is_href_external(href: &str) -> bool {
    href.starts_with("http") || href.starts_with("mailto:")
}
