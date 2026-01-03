use std::{collections::HashMap, path::Path};

use serde_extensions::Overwrite;
use virtual_dom::{parse_html, Document};

use crate::{
    lssg_error::LssgError,
    renderer::RenderContext,
    sitetree::{Page, Resource, SiteId, SiteNode, SiteNodeKind, Stylesheet},
    tree::Dfs,
};

use super::RendererModule;

#[derive(Overwrite, Debug, Default)]
pub struct ExternalModuleOptions {
    href: Option<String>,
}

pub struct ExternalModule {
    external_pages: HashMap<SiteId, Document>,
}

impl Default for ExternalModule {
    fn default() -> Self {
        Self::new()
    }
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
        let pages: Vec<SiteId> = Dfs::new(site_tree)
            .filter(|id| site_tree[*id].kind.is_page())
            .collect();
        for id in pages {
            if let SiteNodeKind::Page(page) = &site_tree[id].kind {
                let options: ExternalModuleOptions = self.options(page);
                if let Some(href) = options.href {
                    let res = reqwest::blocking::get(href)?;
                    let bytes = res.bytes()?;
                    let cursor = std::io::Cursor::new(bytes);
                    let mut zip = zip::ZipArchive::new(cursor)?;
                    for i in 0..zip.len() {
                        let file = zip.by_index(i)?;
                        if file.is_dir() {
                            continue;
                        }
                        if let Some(name) = file.enclosed_name() {
                            let file_name = name.file_name().unwrap().to_str().unwrap();
                            let path = Path::new(file_name);

                            let ancestors: Vec<&Path> = name.ancestors().skip(1).collect();
                            let mut parent_id = id;
                            for ancestor in ancestors.iter().take(ancestors.len().saturating_sub(2)) {
                                let folder_name =
                                    ancestor.file_name().unwrap().to_str().unwrap();
                                // only add if not already present
                                parent_id = match site_tree.get_by_name(folder_name, parent_id) {
                                    Some(id) => *id,
                                    None => site_tree.add(SiteNode::folder(folder_name, parent_id)),
                                }
                            }

                            if let Some(Some("css")) = path.extension().map(|s| s.to_str()) {
                                let sheet = Stylesheet::from_readable(file)?;
                                site_tree.add(SiteNode::stylesheet(file_name, parent_id, sheet));
                                continue;
                            }
                            if "index.html" == file_name {
                                site_tree[parent_id].kind = SiteNodeKind::Page(Page::empty());
                                let document =
                                    Document::from_html(parse_html(file)?).map_err(|e| {
                                        LssgError::new(
                                            e.to_string(),
                                            crate::lssg_error::LssgErrorKind::ParseError,
                                        )
                                    })?;
                                self.external_pages.insert(parent_id, document);
                                continue;
                            }

                            let resource = Resource::from_readable(file)?;
                            site_tree.add(SiteNode::resource(file_name, parent_id, resource));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn render_page<'n>(
        &mut self,
        _document: &mut Document,
        context: &RenderContext<'n>,
    ) -> Option<String> {
        if let Some(doc) = self.external_pages.get(&context.site_id) {
            return Some(doc.to_string());
        }
        None
    }
}
