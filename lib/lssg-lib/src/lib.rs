//! Lyr's Static Site Generator
//!
//! This is a static site generator I wrote mostly for personal use but can also be fitted to work for anyone else.
//!
//!
//! # Examples on how to use this crate
//! ```rs
//! let input = Input::from_string("./test.md")
//! let output = Input::from_string("./build")
//! let mut lssg = Lssg::new(input, output);
//! // Add modules
//! lssg.add_module(ExternalModule::new());
//! lssg.add_module(BlogModule::new());
//! lssg.add_module(DefaultModule::new());
//! // Render code to the folder
//! lssg.render().unwrap()
//! ```
pub mod char_reader;
pub mod lmarkdown;
pub mod parse_error;
pub mod renderer;
pub mod sitetree;

pub mod lssg_error;
mod path_extension;
mod tree;

use std::{
    fs::{create_dir, create_dir_all, remove_dir_all, write},
    path::PathBuf,
};

use log::info;
use lssg_error::LssgError;
use renderer::{Renderer, RendererModule};
use sitetree::Input;

use crate::{
    path_extension::PathExtension,
    sitetree::{Relation, SiteNodeKind, SiteTree},
};

pub struct Lssg {
    input: Input,
    output_directory: PathBuf,
    renderer: Renderer,
}

impl Lssg {
    pub fn new(input: Input, output_directory: PathBuf) -> Lssg {
        let renderer = Renderer::new();
        Lssg {
            input,
            output_directory,
            renderer,
        }
    }

    pub fn add_module(&mut self, module: impl RendererModule + 'static) {
        self.renderer.add_module(module)
    }

    pub fn render(&mut self) -> Result<(), LssgError> {
        info!("Generating SiteTree");
        let mut site_tree = SiteTree::from_input(self.input.clone())?;

        self.renderer.init(&mut site_tree);
        info!("SiteTree:\n{site_tree}");

        // site_tree.minify();

        self.renderer.after_init(&mut site_tree);

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
            queue.append(&mut site_tree[site_id].children.clone());
            let rel_path = site_tree.rel_path(site_tree.root(), site_id);
            let path = self
                .output_directory
                .join(rel_path)
                .canonicalize_nonexistent_path();
            match &mut site_tree[site_id].kind {
                SiteNodeKind::Stylesheet(stylesheet) => {
                    let mut stylesheet = stylesheet.clone();

                    // update resources to stylesheet sitenode path
                    for link in site_tree.links_from(site_id) {
                        if let Relation::Discovered { raw_path } = &link.relation {
                            let updated_resource = site_tree.rel_path(
                                site_tree[site_id]
                                    .parent
                                    .expect("stylesheet must have parent"),
                                link.to,
                            );
                            stylesheet.update_resource(raw_path, &updated_resource);
                        }
                    }

                    stylesheet.write(&path)?;
                }
                SiteNodeKind::Resource(resource) => {
                    resource.write(&path)?;
                }
                SiteNodeKind::Folder => {
                    info!("Creating folder {path:?}",);
                    create_dir(path)?;
                }
                SiteNodeKind::Page { .. } => {
                    let html = self.renderer.render(&site_tree, site_id)?;
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
