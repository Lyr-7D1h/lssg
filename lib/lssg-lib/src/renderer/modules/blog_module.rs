use std::collections::HashMap;

use constants::BLOG_STYLESHEET;
use dates::Dates;
use log::{error, warn};
use proc_virtual_dom::dom;
use rss::RssOptions;
use serde::Deserialize;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    renderer::RenderContext,
    sitetree::{SiteId, SiteNode, SiteNodeKind, SiteTree, Stylesheet},
    tree::DFS,
};
use virtual_dom::{to_attributes, Document, DomNode};

use super::{RendererModule, TokenRenderer};

mod constants;
mod dates;
mod rss;

#[derive(Clone)]
/// Describes the content of a blog post
struct Contents {
    title: Option<String>,
    link: Option<String>,
    description: Option<String>,
}
impl Contents {
    fn empty() -> Self {
        Self {
            title: None,
            link: None,
            description: None,
        }
    }
}

#[derive(Overwrite, Clone, Debug, Deserialize)]
pub struct BlogRootOptions {
    rss: RssOptions,
    /// Use dates from file system to create updated on and modified on tags
    use_fs_dates: bool,
}
impl Default for BlogRootOptions {
    fn default() -> Self {
        Self {
            rss: RssOptions::default(),
            use_fs_dates: true,
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
}
impl Default for BlogPostOptions {
    fn default() -> Self {
        Self {
            render: true,
            modified_on: None,
            created_on: None,
            tags: None,
        }
    }
}

#[derive(Clone)]
struct PostPage {
    post_options: BlogPostOptions,
    /// Relevant dates given by metadata
    dates: Dates,
    /// Contents of the page that gets filled in during render
    contents: Contents,
}

/// Represent a page active in the blog module
struct BlogPage {
    /// Global blog settings applied to all children
    root_options: BlogRootOptions,
    /// Blog Post settings
    post_page: Option<PostPage>,
}

impl BlogPage {
    /// don't render if not a post page or render disabled
    fn should_render(&self) -> Option<&PostPage> {
        let page = self.post_page.as_ref()?;
        if page.post_options.render == false {
            return None;
        }
        Some(page)
    }
}

pub struct BlogModule {
    blog_pages: HashMap<SiteId, BlogPage>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            blog_pages: HashMap::new(),
            has_inserted_date: false,
        }
    }

    /// Get blog page for render
    fn blog_page(&self, site_id: SiteId) -> Option<&PostPage> {
        let page = self.blog_pages.get(&site_id)?;
        let page = page.post_page.as_ref()?;
        if page.post_options.render == false {
            return None;
        }
        Some(page)
    }

    /// Get blog root options
    fn blog_root_options<'n>(
        &self,
        site_tree: &SiteTree,
        site_id: SiteId,
    ) -> Option<&BlogRootOptions> {
        let mut site_id = Some(site_id);
        while let Some(id) = site_id {
            if let Some(page) = self.blog_pages.get(&id) {
                return Some(&page.root_options);
            }

            site_id = site_tree.page_parent(id);
        }
        None
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
        let default_stylesheet = site_tree.add(SiteNode::stylesheet(
            "blog.css",
            site_tree.root(),
            Stylesheet::from_readable(BLOG_STYLESHEET)?,
        ));

        let pages: Vec<usize> = DFS::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();

        // if contains module id it is a blog post
        for site_id in pages {
            match &mut site_tree[site_id].kind {
                SiteNodeKind::Page(page) => {
                    if let Some(Some(table)) = page.attributes().map(|a| a.get(self.id()).cloned())
                    {
                        let root_options = {
                            match table.get("root") {
                                Some(v) => {
                                    let mut o = BlogRootOptions::default();
                                    if let Err(e) = o.overwrite(v.clone()) {
                                        error!(
                                            "Failed to parse options for '{}' module: {e}",
                                            self.id()
                                        )
                                    }
                                    o
                                }
                                None => match self.blog_root_options(site_tree, site_id) {
                                    Some(o) => o.clone(),
                                    // if no blog root parent ignore
                                    None => continue,
                                },
                            }
                        };
                        let post_page = table.get("post").map(|v| {
                            let mut post_options = BlogPostOptions::default();
                            if let Err(e) = post_options.overwrite(v.clone()) {
                                error!("Failed to parse options for '{}' module: {e}", self.id())
                            }

                            let input = if root_options.use_fs_dates {
                                site_tree.get_input(site_id).cloned()
                            } else {
                                None
                            };
                            let dates = Dates::from_post_options(&post_options, &input)
                                .inspect_err(|e| warn!("Failed to parse dates: {e}"))
                                .ok()
                                .unwrap_or_default();

                            PostPage {
                                post_options,
                                dates,
                                contents: Contents::empty(),
                            }
                        });

                        self.blog_pages.insert(
                            site_id,
                            BlogPage {
                                root_options,
                                post_page,
                            },
                        );
                        site_tree.add_link(site_id, default_stylesheet);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render_page<'n>(
        &mut self,
        document: &mut Document,
        context: &RenderContext<'n>,
    ) -> Option<String> {
        let site_id = context.site_id;

        // if not a blog page
        let Some(blog_page) = self.blog_page(site_id) else {
            return None;
        };

        // add article meta data
        if let Some(date) = &blog_page.dates.modified_on {
            let date = date.date.to_rfc3339();
            document
                .head
                .append_child(dom!(<meta property="article:modified_time" content="{date}"/>));
        }
        if let Some(date) = &blog_page.dates.created_on {
            let date = date.date.to_rfc3339();
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
        let Some(blog_page) = self.blog_page(site_id).cloned() else {
            return None;
        };

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                self.has_inserted_date = true;
                let post = document
                    .create_element_with_attributes("div", to_attributes([("class", "post")]));
                let content = document
                    .create_element_with_attributes("div", to_attributes([("class", "content")]));
                post.append_child(content.clone());
                parent.append_child(post);
                // render heading
                tr.render(document, context, content.clone(), &vec![token.clone()]);
                if let Some(date) = blog_page.dates.to_pretty_string() {
                    content.append_child(dom!(<div class="blog__date">{date}</div>));
                }

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
            // TODO add section links
            // Token::Heading { depth, tokens } if *depth == 2 => {
            //     let href = tokens_to_text(tokens).to_lowercase().replace(" ", "-");
            //     println!("{parent_id:?} {token:?}");
            //     let id = tr.render_down(self, dom, context, parent_id, &vec![token.clone()]);
            //     println!("{:?}", dom.get(id));
            //     dom.add_html(id, dom!(r#"<a name="{href}"></a>"#));
            //     dom.add_html(id, dom!(r##"<a class="section-link" aria-hidden="true" href="#{href}"><svg xmlns="http://www.w3.org/2000/svg" height="16" width="20" viewBox="0 0 640 512"><path d="M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l-1.6 1.1c-32.1-22.9-76-19.3-103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1-22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4-6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z"/></svg></a>"##));
            //     return Some(id);
            // }
            _ => {}
        }
        return None;
    }
}

pub fn is_href_external(href: &str) -> bool {
    return href.starts_with("http") || href.starts_with("mailto:");
}
