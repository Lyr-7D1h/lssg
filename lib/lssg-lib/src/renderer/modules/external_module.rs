use std::{collections::HashMap, path::Path};

use serde_extensions::Overwrite;
use virtual_dom::{parse_html, Document};

use crate::{
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Page, SiteId, SiteNode, SiteNodeKind},
    tree::DFS,
};

use super::RendererModule;

#[derive(Overwrite, Debug)]
pub struct ExternalModuleOptions {
    href: Option<String>,
}

impl Default for ExternalModuleOptions {
    fn default() -> Self {
        Self { href: None }
    }
}

pub struct ExternalModule {
    external_pages: HashMap<SiteId, Document>,
}

impl ExternalModule {
    pub fn new() -> Self {
        Self {
            external_pages: HashMap::new(),
        }
    }
}

impl RendererModule for ExternalModule {
    fn id(&self) -> &'static str {
        "external"
    }

    fn init(
        &mut self,
        site_tree: &mut crate::sitetree::SiteTree,
    ) -> Result<(), crate::lssg_error::LssgError> {
        let pages: Vec<usize> = DFS::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();
        for id in pages {
            if let SiteNodeKind::Page(page) = &site_tree[id].kind {
                let options: ExternalModuleOptions = self.options(&page);
                if let Some(href) = options.href {
                    let res = reqwest::blocking::get(href)?;
                    let bytes = res.bytes()?;
                    let cursor = std::io::Cursor::new(bytes);
                    let mut zip = zip::ZipArchive::new(cursor)?;
                    for i in 0..zip.len() {
                        let file = zip.by_index(i)?;
                        if let Some(name) = file.enclosed_name() {
                            let file_name = name.file_name().unwrap().to_str().unwrap();

                            let ancestors: Vec<&Path> = name.ancestors().skip(1).collect();
                            let mut parent_id = id;
                            let has_ancestors = ancestors.len() >= 2;
                            for i in 0..ancestors.len().saturating_sub(2) {
                                println!("ancestors: {:?}", ancestors[i]);
                                parent_id = site_tree.add(SiteNode::folder(
                                    ancestors[i].file_name().unwrap().to_str().unwrap(),
                                    parent_id,
                                ));
                            }

                            // TODO add resources

                            if "index.html" == file_name {
                                let page_id = if has_ancestors {
                                    site_tree.add(SiteNode::page(
                                        file_name,
                                        parent_id,
                                        Page::empty(),
                                    ))
                                } else {
                                    parent_id
                                };
                                let document =
                                    Document::from_html(parse_html(file)?).map_err(|e| {
                                        LssgError::new(
                                            e.to_string(),
                                            crate::lssg_error::LssgErrorKind::ParseError,
                                        )
                                    })?;
                                self.external_pages.insert(page_id, document);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn render_page<'n>(&mut self, document: &mut Document, context: &RenderContext<'n>) {
        if let Some(doc) = self.external_pages.get(&context.site_id) {
            *document = doc.clone();
        }
    }
}
