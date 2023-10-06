pub mod parser;
pub mod renderer;
pub mod sitetree;

mod domtree;
pub mod lssg_error;
mod stylesheet;
mod tree;
mod util;

use std::{
    fs::{copy, create_dir, create_dir_all, remove_dir_all, write},
    path::PathBuf,
};

use log::info;
use lssg_error::LssgError;
use renderer::HtmlRenderer;

use crate::{
    renderer::{BlogModule, DefaultModule, DefaultModuleOptions},
    sitetree::{SiteNodeKind, SiteTree},
    util::canonicalize_nonexistent_path,
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
        let mut renderer = HtmlRenderer::new();
        renderer.add_module(BlogModule::new());
        renderer.add_module(DefaultModule::new());

        info!("Generating SiteMap");
        let mut site_tree = SiteTree::from_index(self.index.clone())?;

        renderer.site_init(&mut site_tree);
        info!("SiteTree:\n{site_tree}");

        if self.output_directory.exists() {
            info!(
                "Removing {:?}",
                canonicalize_nonexistent_path(&self.output_directory)
            );
            remove_dir_all(&self.output_directory)?;
        }
        info!(
            "Creating {:?}",
            canonicalize_nonexistent_path(&self.output_directory)
        );
        create_dir_all(&self.output_directory)?;

        let mut queue: Vec<usize> = vec![site_tree.root()];
        while let Some(site_id) = queue.pop() {
            let node = site_tree.get(site_id)?;
            queue.append(&mut node.children.clone());
            let path = self.output_directory.join(site_tree.path(site_id));
            match &node.kind {
                SiteNodeKind::Stylesheet(s) => {
                    info!("Writing stylesheet {path:?}",);
                    write(path, s.to_string())?;
                }
                SiteNodeKind::Resource { input } => {
                    copy(input, path)?;
                }
                SiteNodeKind::Folder => {
                    create_dir(path)?;
                }
                SiteNodeKind::Page { keep_name, .. } => {
                    let html = renderer.render(&site_tree, site_id)?;
                    let html_output_path = if *keep_name {
                        canonicalize_nonexistent_path(&path.join(format!("../{}.html", node.name)))
                    } else {
                        create_dir_all(&path)?;
                        canonicalize_nonexistent_path(&path.join("index.html"))
                    };
                    info!(
                        "Writing to {:?}",
                        canonicalize_nonexistent_path(&html_output_path)
                    );
                    write(html_output_path, html)?;
                }
            }
        }

        info!("All files written");

        Ok(())
    }
}
