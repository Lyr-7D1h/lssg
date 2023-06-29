pub mod parser;
pub mod renderer;
pub mod sitemap;
mod stylesheet;

use std::{
    fs::{copy, create_dir, create_dir_all, read_to_string, remove_dir_all, write, File},
    io::{self},
    path::{Path, PathBuf},
};

use log::info;
use parser::{lexer::Token, parse_error::ParseError, Parser};
use renderer::{HtmlDocumentRenderOptions, HtmlLink, HtmlRenderer, Meta, Rel};
use sitemap::SiteMap;

use crate::stylesheet::Stylesheet;

#[derive(Debug)]
pub enum LssgError {
    ParseError(ParseError),
    Regex(regex::Error),
    Render(String),
    Io(io::Error),
}
impl LssgError {
    pub fn render(error: &str) -> LssgError {
        LssgError::Render(error.into())
    }
}
impl From<ParseError> for LssgError {
    fn from(error: ParseError) -> Self {
        Self::ParseError(error)
    }
}
impl From<io::Error> for LssgError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<regex::Error> for LssgError {
    fn from(error: regex::Error) -> Self {
        Self::Regex(error)
    }
}

#[derive(Debug)]
pub struct Link {
    pub rel: Rel,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct LssgOptions {
    pub index: PathBuf,
    pub output_directory: PathBuf,
    pub global_stylesheet: Option<PathBuf>,
    /// Overwrite the default stylesheet with your own
    pub overwrite_default_stylesheet: bool,
    /// Add extra resources
    pub links: Vec<Link>,
    pub title: String,
    /// Translates to meta tags <https://www.w3schools.com/tags/tag_meta.asp>
    pub keywords: Vec<(String, String)>,
    /// Lang attribute ("en") <https://www.w3schools.com/tags/ref_language_codes.asp>
    pub language: String,
}

pub struct Lssg {
    options: LssgOptions,
}

impl Lssg {
    pub fn new(options: LssgOptions) -> Lssg {
        Lssg { options }
    }

    pub fn preview(&self, port: u32) {
        info!("Listing on 0.0.0.0:{port}");
        todo!()
    }

    pub fn render(&self) -> Result<(), LssgError> {
        let mut stylesheet = if let Some(p) = &self.options.global_stylesheet {
            let mut s = if self.options.overwrite_default_stylesheet {
                Stylesheet::new()
            } else {
                Stylesheet::default()
            };
            s.append(p)?;
            s
        } else {
            Stylesheet::default()
        };
        for l in self.options.links.iter() {
            if let Rel::Stylesheet = l.rel {
                stylesheet.append(&l.path)?;
            }
        }

        let mut site_map = SiteMap::from_index(self.options.index.clone())?;
        let stylesheet_id =
            site_map.add_stylesheet("main.css".into(), stylesheet, site_map.root())?;

        info!("SiteMap:\n{site_map}");

        let render_options = HtmlDocumentRenderOptions {
            links: vec![],
            title: self.options.title.clone(),
            meta: self
                .options
                .keywords
                .iter()
                .map(|(name, content)| Meta {
                    name: name.clone(),
                    content: content.clone(),
                })
                .collect(),
            language: self.options.language.clone(),
        };

        info!("Removing {:?}", self.options.output_directory);
        remove_dir_all(&self.options.output_directory)?;
        info!("Creating {:?}", self.options.output_directory);
        create_dir_all(&self.options.output_directory)?;

        let mut queue: Vec<usize> = vec![site_map.root()];
        let renderer = HtmlRenderer::new(&site_map);
        while let Some(id) = queue.pop() {
            let node = site_map.get(id)?;
            queue.append(&mut node.children.clone());
            let path = self.options.output_directory.join(site_map.path(id));
            match &node.node_type {
                sitemap::NodeType::Stylesheet(s) => {
                    info!("Writing concatinated stylesheet {path:?}",);
                    write(path, s.to_string())?;
                }
                sitemap::NodeType::Resource { input } => {
                    copy(input, path)?;
                }
                sitemap::NodeType::Folder => {
                    create_dir(path)?;
                }
                sitemap::NodeType::Page { .. } => {
                    let mut options = render_options.clone();
                    options.links.push(HtmlLink {
                        rel: renderer::Rel::Stylesheet,
                        href: site_map.rel_path(id, stylesheet_id),
                    });
                    let html = renderer.render(id, options)?;
                    create_dir_all(&path)?;
                    let html_output_path = path.join("index.html");
                    info!("Writing to {html_output_path:?}");
                    write(html_output_path, html)?;
                }
            }
        }

        Ok(())
    }
}
