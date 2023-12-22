use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    html,
    html::{to_attributes, DomId, DomTree},
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Input, SiteNodeKind},
    tree::DFS,
};

use super::{RendererModule, TokenRenderer};

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

    fn after_init(
        &mut self,
        site_tree: &crate::sitetree::SiteTree,
    ) -> Result<(), crate::lssg_error::LssgError> {
        // if parent contains blog key than all children also belong to blog
        for id in DFS::new(site_tree) {
            match &site_tree[id].kind {
                SiteNodeKind::Page(page) => {
                    let options: BlogOptions = self.options(page);

                    if options.root {
                        self.root_site_ids.insert(id);
                        continue;
                    }

                    if let Some(parent) = site_tree.page_parent(id) {
                        if self.post_site_ids.contains(&parent)
                            || self.root_site_ids.contains(&parent)
                        {
                            self.post_site_ids.insert(id);
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

        // TODO make shorter
        let body = dom.get_elements_by_tag_name("body")[0];

        // add breacrumbs
        {
            let nav = dom.add_element_with_attributes(
                body,
                "nav",
                to_attributes([("class", "breadcrumbs")]),
            );

            dom.add_text(nav, "/");

            let parents = site_tree.parents(site_id);
            let parents_length = parents.len();
            for (i, p) in parents.into_iter().rev().enumerate() {
                let a = dom.add_element_with_attributes(
                    nav,
                    "a",
                    to_attributes([("href", site_tree.rel_path(site_id, p))]),
                );
                if i != parents_length - 1 {
                    dom.add_text(nav, "/");
                }
                dom.add_text(a, site_tree[p].name.clone());
            }
            dom.add_text(nav, format!("/{}", site_tree[site_id].name));
        }
    }

    fn render_body<'n>(
        &mut self,
        dom: &mut DomTree,
        context: &RenderContext<'n>,
        parent_id: DomId,
        token: &Token,
        tr: &mut TokenRenderer,
    ) -> Option<DomId> {
        let site_id = context.site_id;
        if !self.post_site_ids.contains(&site_id) {
            return None;
        }

        match token {
            Token::Heading { depth, .. } if *depth == 1 && !self.has_inserted_date => {
                match get_date(self, context) {
                    Ok(date) => {
                        self.has_inserted_date = true;
                        let post = dom.add_element_with_attributes(
                            parent_id,
                            "div",
                            to_attributes([("class", "post")]),
                        );
                        let content = dom.add_element_with_attributes(
                            post,
                            "div",
                            to_attributes([("class", "content")]),
                        );
                        // render heading
                        tr.render(dom, context, content, &vec![token.clone()]);
                        dom.add_html(
                            content,
                            html!(r#"<div class="post-updated-on">{date}</div>"#),
                        );

                        return Some(content);
                    }
                    Err(e) => error!("failed to read date from post: {e}"),
                }
            }
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
