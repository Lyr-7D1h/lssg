pub mod lmarkdown;
pub mod renderer;
pub mod sitetree;

mod domtree;
pub mod lssg_error;
mod path_extension;
mod stylesheet;
mod tree;

use std::{
    fs::{copy, create_dir, create_dir_all, remove_dir_all, write},
    path::{Path, PathBuf},
};

use log::info;
use lssg_error::LssgError;
use renderer::Renderer;

use crate::{
    path_extension::PathExtension,
    renderer::{BlogModule, DefaultModule},
    sitetree::{Relation, SiteNodeKind, SiteTree},
};

#[derive(Debug, Clone)]
pub struct LssgOptions {
    pub index: PathBuf,
    pub output_directory: PathBuf,
}

pub struct Lssg {
    index: PathBuf,
    output_directory: PathBuf,
}

impl Lssg {
    pub fn new(options: LssgOptions) -> Lssg {
        let LssgOptions {
            index,
            output_directory,
        } = options;
        Lssg {
            index,
            output_directory,
        }
    }

    pub fn render(&self) -> Result<(), LssgError> {
        let mut renderer = Renderer::new();
        renderer.add_module(BlogModule::new());
        renderer.add_module(DefaultModule::new());

        info!("Generating SiteTree");
        let mut site_tree = SiteTree::from_index(self.index.clone())?;

        renderer.init(&mut site_tree);
        info!("SiteTree:\n{site_tree}");

        renderer.after_init(&mut site_tree);

        if self.output_directory.exists() {
            info!(
                "Removing {:?}",
                self.output_directory.canonicalize_nonexistent_path()
            );
            remove_dir_all(&self.output_directory)?;
        }
        info!(
            "Creating {:?}",
            self.output_directory.canonicalize_nonexistent_path()
        );
        create_dir_all(&self.output_directory)?;

        let mut queue: Vec<usize> = vec![site_tree.root()];
        while let Some(site_id) = queue.pop() {
            let node = site_tree.get(site_id)?;
            queue.append(&mut node.children.clone());
            let path = self.output_directory.join(site_tree.path(site_id));
            match &node.kind {
                SiteNodeKind::Stylesheet { stylesheet } => {
                    let mut stylesheet = stylesheet.clone();
                    for link in site_tree.links_from(site_id) {
                        if let Relation::Discovered { path } = &link.relation {
                            let updated_resource = site_tree.rel_path(site_id, link.to);
                            (&mut stylesheet).update_resource(Path::new(path), updated_resource);
                        }
                    }
                    info!("Writing stylesheet {path:?}",);
                    write(path, stylesheet.to_string())?;
                }
                SiteNodeKind::Resource { input } => {
                    info!("Writing resource {path:?}",);
                    copy(input, path)?;
                }
                SiteNodeKind::Folder => {
                    info!("Creating folder {path:?}",);
                    create_dir(path)?;
                }
                SiteNodeKind::Page { .. } => {
                    let html = renderer.render(&site_tree, site_id)?;
                    create_dir_all(&path)?;
                    let html_output_path = path.join("index.html").canonicalize_nonexistent_path();

                    info!(
                        "Writing to {:?}",
                        (&html_output_path).canonicalize_nonexistent_path()
                    );
                    write(html_output_path, html)?;
                }
            }
        }

        info!("All files written");

        Ok(())
    }
}
