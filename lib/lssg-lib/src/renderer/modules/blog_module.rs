use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, Utc};
use log::{error, warn};
use serde_extensions::Overwrite;

use crate::{
    html::{to_attributes, DomId, DomTree},
    lmarkdown::Token,
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Input, SiteNodeKind},
    tree::DFS,
};

use super::{RendererModule, TokenRenderer};

pub struct BlogModule {
    enabled_site_ids: HashSet<usize>,
    /// Local variable to keep track if date has been inserted
    has_inserted_date: bool,
}

impl BlogModule {
    pub fn new() -> Self {
        Self {
            enabled_site_ids: HashSet::new(),
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
                SiteNodeKind::Page { page, .. } => {
                    if let Some(attributes) = page.attributes() {
                        if let Some(_) = attributes.get("blog") {
                            self.enabled_site_ids.insert(id);
                            continue;
                        }
                    }

                    if let Some(parent) = site_tree.page_parent(id) {
                        if self.enabled_site_ids.contains(&parent) {
                            self.enabled_site_ids.insert(id);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn render_page<'n>(&mut self, dom: &mut DomTree, context: &RenderContext<'n>) {
        let site_tree = context.site_tree();
        let site_id = context.site_id();

        if !self.enabled_site_ids.contains(&site_id) {
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
    ) -> bool {
        let site_id = context.site_id();
        if !self.enabled_site_ids.contains(&site_id) {
            return false;
        }

        match token {
            Token::Heading { depth, tokens } if *depth == 1 && !self.has_inserted_date => {
                match get_date(self, context) {
                    Ok(date) => {
                        self.has_inserted_date = true;
                        // render heading
                        tr.render(dom, context, parent_id, &vec![token.clone()]);
                        let div = dom.add_element_with_attributes(
                            parent_id,
                            "div",
                            to_attributes([("class", "post-updated-on")]),
                        );
                        dom.add_text(div, date);

                        return true;
                    }
                    Err(e) => error!("failed to read date from post: {e}"),
                }
            }
            _ => {}
        }
        return false;
    }
}

#[derive(Overwrite)]
pub struct PostOptions {
    modified_on: Option<String>,
}
impl Default for PostOptions {
    fn default() -> Self {
        Self { modified_on: None }
    }
}

/// get the date from input and options
fn get_date(module: &mut BlogModule, context: &RenderContext) -> Result<String, LssgError> {
    let po: PostOptions = module.options(context.page());

    if let Some(date) = po.modified_on {
        match DateTime::<Utc>::from_str(&date) {
            Ok(date) => {
                let date = date.format("Updated on %B %d, %Y").to_string();
                return Ok(date);
            }
            Err(e) => warn!("could not parse modified_on to date: {e}"),
        }
    }

    context.page().attributes();
    match context.input {
        Input::Local { path } => {
            let date: DateTime<Utc> = path.metadata()?.modified()?.into();
            let date = date.format("Updated on %B %d, %Y").to_string();
            Ok(date)
        }
        Input::External { url } => {
            return Err(LssgError::render(
                "getting modified date from url is not supported",
            ))
        }
    }
}
