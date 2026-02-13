use std::collections::HashMap;

use proc_virtual_dom::dom;

use crate::{
    lmarkdown::Token,
    renderer::{
        InitContext, RenderContext,
        modules::post_module::{constants::POST_STYLESHEET, post_page::PostPage, rss::RssOptions},
    },
    sitetree::{Relation, SiteId, SiteNode, Stylesheet},
};
use virtual_dom::{Document, DomNode};

use super::{RendererModule, TokenRenderer};

mod constants;
mod post_dates;
mod post_page;
mod rss;

#[derive(Default)]
pub struct PostModule {
    posts: HashMap<SiteId, PostPage>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl PostModule {
    /// Get post page for render
    fn post_page(&self, site_id: SiteId) -> Option<&PostPage> {
        let page = self.posts.get(&site_id)?;
        if !page.options.render {
            return None;
        }
        Some(page)
    }
}

impl RendererModule for PostModule {
    fn id(&self) -> &'static str {
        "post"
    }

    fn init(
        &mut self,
        InitContext { site_tree, .. }: InitContext,
    ) -> Result<(), crate::lssg_error::LssgError> {
        let posts = self.collect_post_pages(site_tree);
        if posts.len() > 0 {
            let default_stylesheet = site_tree.add(SiteNode::stylesheet(
                "post.css",
                site_tree.root(),
                Stylesheet::from_readable(POST_STYLESHEET)?,
            ));
            for page_id in posts.keys() {
                site_tree.add_link(*page_id, default_stylesheet, Relation::External);
            }
        }

        // TODO: move to a separate module
        // rss
        for (id, options) in site_tree
            .pages()
            .filter_map(|(id, page)| {
                Some((id, self.options_with_module_id::<RssOptions>(page, "rss")?))
            })
            .collect::<Vec<_>>()
        {
            let posts: Vec<_> = site_tree
                .children(id)
                .filter_map(|id| posts.get(&id).map(|p| (id, p)))
                .collect();
            let Some(rss_feed) = rss::RssFeed::from_root(id, posts, site_tree, options.clone())
            else {
                continue;
            };
            let rss_content = rss_feed.to_string();

            let rss_resource = crate::sitetree::Resource::new_static(rss_content);
            let rss_filename = options
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("feed.xml");

            site_tree.add(SiteNode::resource(rss_filename, id, rss_resource));
        }

        self.posts = posts;

        Ok(())
    }

    fn render_page<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
    ) -> Option<String> {
        let site_id = context.site_id;

        // if not a post page
        let post_page = self.post_page(site_id)?;

        // add article meta data
        if let Some(date) = &post_page.dates.modified_on {
            let date = date.to_rfc3339();
            document
                .head
                .append_child(dom!(<meta property="article:modified_time" content="{date}"/>));
        }
        if let Some(date) = &post_page.dates.created_on {
            let date = date.to_rfc3339();
            document.head.append_child(dom!(
                <meta property="article:published_time" content="{date}"/>
            ));
        }

        // reset state
        self.has_inserted_date = false;

        None
    }

    fn render_token<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
        parent: DomNode,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomNode> {
        let site_id = context.site_id;

        // if not a post page
        let dates = self.post_page(site_id)?.dates.clone();

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
                if let Some(date) = dates.to_pretty_string() {
                    parent.append_child(dom!(<p class="post__date">{date}</p>));
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
