use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    dom::{to_attributes, DomNode, DomTree},
    html,
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Input, SiteNode, SiteNodeKind, Stylesheet},
    tree::{Node, Tree, DFS},
};

use super::{tokens_to_text, RendererModule, TokenRenderer};

const BLOG_STYLESHEET: &[u8] = include_bytes!("./blog_stylesheet.css");

#[derive(Overwrite, Debug)]
pub struct BlogOptions {
    root: bool,
    /// When has an article been changed (%Y-%m-%d)
    modified_on: Option<String>,
}
impl Default for BlogOptions {
    fn default() -> Self {
        Self {
            root: false,
            modified_on: None,
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
        return "blog";
    }

    fn init(
        &mut self,
        site_tree: &mut crate::sitetree::SiteTree,
    ) -> Result<(), crate::lssg_error::LssgError> {
        let default_stylesheet = site_tree.add(SiteNode::stylesheet(
            "blog.css",
            site_tree.root(),
            Stylesheet::from_readable(BLOG_STYLESHEET)?,
        ))?;

        let pages: Vec<usize> = DFS::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();

        // if parent contains blog key than all children also belong to blog
        for site_id in pages {
            match &site_tree[site_id].kind {
                SiteNodeKind::Page(page) => {
                    let options: BlogOptions = self.options(page);

                    if options.root {
                        self.root_site_ids.insert(site_id);
                        site_tree.add_link(site_id, default_stylesheet);
                        continue;
                    }

                    if let Some(parent) = site_tree.page_parent(site_id) {
                        if self.post_site_ids.contains(&parent)
                            || self.root_site_ids.contains(&parent)
                        {
                            self.post_site_ids.insert(site_id);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render_page<'n>(&mut self, dom: &mut DomTree, context: &RenderContext<'n>) {
        let site_tree = context.site_tree;
        let site_id = context.site_id;

        if !self.post_site_ids.contains(&site_id) && !self.root_site_ids.contains(&site_id) {
            return;
        }

        // reset state
        self.has_inserted_date = false;

        let body = dom.body();

        // add breacrumbs
        {
            let nav = dom
                .create_element_with_attributes("nav", to_attributes([("class", "breadcrumbs")]));

            nav.append_child(dom.create_text_node("/"));

            let parents = site_tree.parents(site_id);
            let parents_length = parents.len();
            for (i, p) in parents.into_iter().rev().enumerate() {
                let a = dom.create_element_with_attributes(
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                );
                a.append_child(dom.create_text_node(site_tree[p].name.clone()));
                nav.append_child(a);
                if i != parents_length - 1 {
                    nav.append_child(dom.create_text_node("/"));
                }
            }
            nav.append_child(dom.create_text_node(format!("/{}", site_tree[site_id].name)));

            body.append_child(nav);
        }
    }

    fn render_body<'n>(
        &mut self,
        dom: &mut DomTree,
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
                        let post = dom.create_element_with_attributes(
                            "div",
                            to_attributes([("class", "post")]),
                        );
                        let content = dom.create_element_with_attributes(
                            "div",
                            to_attributes([("class", "content")]),
                        );
                        post.append_child(content.clone());
                        parent.append_child(post);
                        // render heading
                        tr.render(dom, context, content.clone(), &vec![token.clone()]);
                        content.append_child(html!(r#"<div class="post-updated-on">{date}</div>"#));

                        return Some(content);
                    }
                    Err(e) => error!("failed to read date from post: {e}"),
                }
            }
            // TODO add section links
            // Token::Heading { depth, tokens } if *depth == 2 => {
            //     let href = tokens_to_text(tokens).to_lowercase().replace(" ", "-");
            //     println!("{parent_id:?} {token:?}");
            //     let id = tr.render_down(self, dom, context, parent_id, &vec![token.clone()]);
            //     println!("{:?}", dom.get(id));
            //     dom.add_html(id, html!(r#"<a name="{href}"></a>"#));
            //     dom.add_html(id, html!(r##"<a class="section-link" aria-hidden="true" href="#{href}"><svg xmlns="http://www.w3.org/2000/svg" height="16" width="20" viewBox="0 0 640 512"><path d="M579.8 267.7c56.5-56.5 56.5-148 0-204.5c-50-50-128.8-56.5-186.3-15.4l-1.6 1.1c-14.4 10.3-17.7 30.3-7.4 44.6s30.3 17.7 44.6 7.4l1.6-1.1c32.1-22.9 76-19.3 103.8 8.6c31.5 31.5 31.5 82.5 0 114L422.3 334.8c-31.5 31.5-82.5 31.5-114 0c-27.9-27.9-31.5-71.8-8.6-103.8l1.1-1.6c10.3-14.4 6.9-34.4-7.4-44.6s-34.4-6.9-44.6 7.4l-1.1 1.6C206.5 251.2 213 330 263 380c56.5 56.5 148 56.5 204.5 0L579.8 267.7zM60.2 244.3c-56.5 56.5-56.5 148 0 204.5c50 50 128.8 56.5 186.3 15.4l1.6-1.1c14.4-10.3 17.7-30.3 7.4-44.6s-30.3-17.7-44.6-7.4l-1.6 1.1c-32.1 22.9-76 19.3-103.8-8.6C74 372 74 321 105.5 289.5L217.7 177.2c31.5-31.5 82.5-31.5 114 0c27.9 27.9 31.5 71.8 8.6 103.9l-1.1 1.6c-10.3 14.4-6.9 34.4 7.4 44.6s34.4 6.9 44.6-7.4l1.1-1.6C433.5 260.8 427 182 377 132c-56.5-56.5-148-56.5-204.5 0L60.2 244.3z"/></svg></a>"##));
            //     return Some(id);
            // }
            _ => {}
        }
        return None;
    }
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
