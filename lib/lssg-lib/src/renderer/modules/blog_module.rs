use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use log::{error, warn};
use proc_virtual_dom::dom;
use serde_extensions::Overwrite;

use crate::{
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Input, SiteNode, SiteNodeKind, Stylesheet},
    tree::DFS,
};
use virtual_dom::{to_attributes, Document, DomNode};

use super::{RendererModule, TokenRenderer};

const BLOG_STYLESHEET: &[u8] = include_bytes!("./blog_stylesheet.css");

#[derive(Overwrite, Debug)]
pub struct BlogOptions {
    root: bool,
    /// When has an article been changed (%Y-%m-%d)
    modified_on: Option<String>,
    tags: Option<Vec<String>>,
}
impl Default for BlogOptions {
    fn default() -> Self {
        Self {
            root: false,
            modified_on: None,
            tags: None,
        }
    }
}

pub struct BlogModule {
    post_site_ids: HashSet<usize>,
    root_site_ids: HashSet<usize>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            post_site_ids: HashSet::new(),
            root_site_ids: HashSet::new(),
            has_inserted_date: false,
        }
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

        // if parent contains blog key than all children also belong to blog
        for site_id in pages {
            // if parent is a blog post than this is also a blog post
            if let Some(parent) = site_tree.page_parent(site_id) {
                if self.post_site_ids.contains(&parent) || self.root_site_ids.contains(&parent) {
                    self.post_site_ids.insert(site_id);
                }
            }

            match &mut site_tree[site_id].kind {
                SiteNodeKind::Page(page) => {
                    let options: BlogOptions = self.options(page);

                    if options.root {
                        self.root_site_ids.insert(site_id);
                        site_tree.add_link(site_id, default_stylesheet);
                        continue;
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

        if !self.post_site_ids.contains(&site_id) && !self.root_site_ids.contains(&site_id) {
            return None;
        }

        // add article meta data
        let options: BlogOptions = self.options(context.page);
        if let Some(date) = options.modified_on {
            document
                .head
                .append_child(dom!(<meta property="article:modified_time" content="{date}"/>));
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
        if !self.post_site_ids.contains(&site_id) {
            return None;
        }

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                match get_date(self, context) {
                    Ok(date) => {
                        self.has_inserted_date = true;
                        let post = document.create_element_with_attributes(
                            "div",
                            to_attributes([("class", "post")]),
                        );
                        let content = document.create_element_with_attributes(
                            "div",
                            to_attributes([("class", "content")]),
                        );
                        post.append_child(content.clone());
                        parent.append_child(post);
                        // render heading
                        tr.render(document, context, content.clone(), &vec![token.clone()]);
                        content.append_child(dom!(<div class="post-updated-on">{date}</div>));

                        return Some(content);
                    }
                    Err(e) => error!("failed to read date from post: {e}"),
                }
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
            //     dom.add_html(id, dom!(r##"<a class="section-link" aria-hidden="true" href="#{href}"><svg xmlns="http://www.w3.org/2000/svg" height="16" width="20" viewBox="0 0 640 512"><path d="M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l1.6-1.1c32.1-22.9 76-19.3 103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1 22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4 6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z"/></svg></a>"##));
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

/// get the date from input and options
fn get_date(module: &mut BlogModule, context: &RenderContext) -> Result<String, LssgError> {
    let po: BlogOptions = module.options(context.page);

    if let Some(date) = po.modified_on {
        match NaiveDate::from_str(&date) {
            Ok(date) => {
                let date = date.format("Updated on %B %d, %Y").to_string();
                return Ok(date);
            }
            Err(e) => warn!("could not parse modified_on to date: {e}"),
        }
    }

    match context.input {
        Some(Input::Local { path }) => {
            let date: DateTime<Utc> = path.metadata()?.modified()?.into();
            let date = date.format("Updated on %B %d, %Y").to_string();
            Ok(date)
        }
        Some(Input::External { .. }) => {
            return Err(LssgError::render(
                "getting modified date from url is not supported",
            ))
        }
        None => return Err(LssgError::render("page does not have an Input")),
    }
}
