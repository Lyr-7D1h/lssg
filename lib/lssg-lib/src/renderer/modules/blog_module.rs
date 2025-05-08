use std::collections::HashSet;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use constants::BLOG_STYLESHEET;
use log::warn;
use proc_virtual_dom::dom;
use rss::RssOptions;
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

mod constants;
mod rss;

#[derive(Overwrite, Debug)]
pub struct BlogOptions {
    rss: RssOptions,
    /// When has an article been changed (%Y-%m-%d)
    modified_on: Option<String>,
    created_on: Option<String>,
    tags: Option<Vec<String>>,
}
impl Default for BlogOptions {
    fn default() -> Self {
        Self {
            modified_on: None,
            created_on: None,
            rss: RssOptions::default(),
            tags: None,
        }
    }
}

pub struct BlogModule {
    site_ids: HashSet<usize>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            site_ids: HashSet::new(),
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

        // if contains module id it is a blog post
        for site_id in pages {
            match &mut site_tree[site_id].kind {
                SiteNodeKind::Page(page) => {
                    if let Some(attributes) = page.attributes() {
                        if attributes.contains_key(self.id()) {
                            self.site_ids.insert(site_id);
                            site_tree.add_link(site_id, default_stylesheet);
                        }
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
        if !self.site_ids.contains(&site_id) {
            return None;
        }

        // add article meta data
        let options: BlogOptions = self.options(context.page);
        if let Some(date) = options.modified_on {
            document
                .head
                .append_child(dom!(<meta property="article:modified_time" content="{date}"/>));
        }
        if let Some(date) = options.created_on {
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
        if !self.site_ids.contains(&site_id) {
            return None;
        }

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                let options: BlogOptions = self.options(context.page);
                let dates = match Dates::from_options(&options, context) {
                    Ok(dates) => dates,
                    Err(e) => {
                        warn!("Failed to get dates: {e}");
                        return None;
                    }
                };
                self.has_inserted_date = true;
                let post = document
                    .create_element_with_attributes("div", to_attributes([("class", "post")]));
                let content = document
                    .create_element_with_attributes("div", to_attributes([("class", "content")]));
                post.append_child(content.clone());
                parent.append_child(post);
                // render heading
                tr.render(document, context, content.clone(), &vec![token.clone()]);
                if let Some(date) = dates.to_pretty_string() {
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

struct Dates {
    modified_on: Option<DateTime<Utc>>,
    created_on: Option<DateTime<Utc>>,
}
impl Dates {
    fn from_options(options: &BlogOptions, context: &RenderContext) -> Result<Self, LssgError> {
        let created_on = match options
            .created_on
            .as_ref()
            .map(|s| {
                parse_date_string(&s)
                    .inspect_err(|e| {
                        warn!("Failed to parse created on '{s}': {e}");
                    })
                    .ok()
            })
            .flatten()
        {
            Some(date) => Some(date),
            None => match context.input {
                Some(Input::Local { path }) => Some(path.metadata()?.created()?.into()),
                _ => None,
            },
        };
        let modified_on = match options
            .modified_on
            .as_ref()
            .map(|s| {
                parse_date_string(s)
                    .inspect_err(|e| {
                        warn!("Failed to parse modified on '{s}': {e}");
                    })
                    .ok()
            })
            .flatten()
        {
            Some(date) => Some(date),
            None => match context.input {
                Some(Input::Local { path }) => Some(path.metadata()?.modified()?.into()),
                _ => None,
            },
        };

        Ok(Self {
            created_on,
            modified_on,
        })
    }

    fn to_pretty_string(&self) -> Option<String> {
        if let Some(date) = self.modified_on {
            return Some(date.format("Updated on %B %d, %Y").to_string());
        }
        if let Some(date) = self.created_on {
            return Some(date.format("Created on %B %d, %Y").to_string());
        }
        None
    }
}
fn parse_date_string(input: &String) -> Result<DateTime<Utc>, LssgError> {
    // Try RFC 3339 first (includes timezone): "2025-05-08T14:30:00+02:00"
    if let Ok(dt_fixed) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt_fixed.with_timezone(&Utc));
    }

    // Try full datetime without timezone: "2025-05-08T14:30:00"
    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(input, "%Y-%m-%dT%H:%M:%S") {
        return Ok(Utc.from_utc_datetime(&naive_dt));
    }

    // Try date-only formats
    for format in ["%Y-%m-%e", "%Y-%m-%d"] {
        if let Ok(naive_date) = NaiveDate::parse_from_str(input, format) {
            // Use modern chrono method for creating time
            let naive_time = chrono::NaiveTime::from_hms_opt(0, 0, 0)
                .ok_or_else(|| LssgError::parse(format!("Date out of range: {input}")))?;
            let naive_dt = naive_date.and_time(naive_time);
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
    }

    // If none match, return an error
    Err(LssgError::parse(format!("Unknown date format: {input}")))
}
