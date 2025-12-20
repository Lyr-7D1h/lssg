use std::collections::HashMap;

use proc_virtual_dom::dom;
use rss::RssOptions;
use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    renderer::{
        modules::blog_module::{
            collect_roots::{PostPage, RootPage},
            constants::BLOG_STYLESHEET,
        },
        RenderContext,
    },
    sitetree::{SiteId, SiteNode, Stylesheet},
};
use virtual_dom::{to_attributes, Document, DomNode};

use super::{RendererModule, TokenRenderer};

mod blog_post_dates;
mod collect_roots;
mod constants;
mod rss;

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub struct BlogRootOptions {
    rss: RssOptions,
    /// Use dates from file system to create updated on and modified on tags
    /// by default false
    use_fs_dates: bool,
}
impl Default for BlogRootOptions {
    fn default() -> Self {
        Self {
            rss: RssOptions::default(),
            use_fs_dates: false,
        }
    }
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

pub struct BlogModule {
    roots: HashMap<SiteId, RootPage>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            has_inserted_date: false,
            roots: HashMap::new(),
        }
    }

    /// Get blog page for render
    fn post_page(&self, site_id: SiteId) -> Option<&PostPage> {
        let page = self
            .roots
            .values()
            .find_map(|root| root.posts.get(&site_id))?;
        if page.post_options.render == false {
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
                site_tree.add_link(*page_id, default_stylesheet);
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
        let Some(blog_page) = self.post_page(site_id) else {
            return None;
        };

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
        let Some(blog_page) = self.post_page(site_id).cloned() else {
            return None;
        };

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                self.has_inserted_date = true;
                let content = document.create_element_with_attributes(
                    "div",
                    to_attributes([("id", "blog__content")]),
                );
                // render heading
                tr.render_down(
                    self,
                    document,
                    context,
                    content.clone(),
                    &vec![token.clone()],
                );
                if let Some(date) = blog_page.dates.to_pretty_string() {
                    content.append_child(dom!(<div class="blog__date">{date}</div>));
                }
                parent.append_child(content.clone());
                return Some(content);
            }
            Token::Link {
                tokens: text, href, ..
            } => {
                if text.len() == 0 {
                    return Some(parent);
                }

                // add icon if external link
                if is_href_external(href) {
                    tr.render_down(
                        self,
                        document,
                        context,
                        parent.clone(),
                        &vec![token.clone()],
                    );
                    parent.append_child(dom!(<svg width="1em" height="1em" viewBox="0 0 24 24" style="cursor:pointer"><g stroke-width="2.1" stroke="#666" fill="none" stroke-linecap="round" stroke-linejoin="round"><polyline points="17 13.5 17 19.5 5 19.5 5 7.5 11 7.5"></polyline><path d="M14,4.5 L20,4.5 L20,10.5 M20,4.5 L11,13.5"></path></g></svg>));
                    return Some(parent);
                }
            }
            _ => {}
        }
        return None;
    }

    fn after_render<'n>(&mut self, document: &mut Document, _context: &RenderContext<'n>) {
        // Add link icons to each sub header
        if let Some(post) = document.body.get_element_by_id("blog__post") {
            for mut heading in post.get_elements_by_tag_name("h2") {
                let id = match heading.get_attribute("id") {
                    Some(id) => id,
                    None => {
                        let id = heading.inner_text().to_ascii_lowercase().replace(" ", "-");
                        heading.set_attribute("id", &id);
                        id
                    }
                };
                heading.prepend(
                    dom!(
                    <a href="#{id}" class="section-link" aria-hidden=true>
                        <svg xmlns="http://www.w3.org/2000/svg" height="16" width="20" viewBox="0 0 640 512"><path d="M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l-1.6 1.1c-32.1-22.9-76-19.3-103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1-22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4-6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z"/></svg>
                    </a>)

                );
            }
        }
    }
}

pub fn is_href_external(href: &str) -> bool {
    return href.starts_with("http") || href.starts_with("mailto:");
}
