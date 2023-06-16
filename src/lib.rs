pub mod parser;
pub mod renderer;
pub mod sitemap;
mod stylesheet;

use std::{
    fs::{copy, create_dir_all, read_to_string, remove_dir_all, write, File},
    io::{self},
    path::{Path, PathBuf},
};

use log::info;
use parser::{lexer::Token, parse_error::ParseError, Parser};
use renderer::{HtmlDocument, HtmlDocumentRenderOptions, HtmlLink, Meta, Rel};
use sitemap::SiteMap;

use crate::stylesheet::Stylesheet;

#[derive(Debug)]
pub enum LssgError {
    ParseError(ParseError),
    Regex(regex::Error),
    Io(io::Error),
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
    /// Overwrite the default stylesheet with your own
    pub global_stylesheet: Option<PathBuf>,
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

    pub fn stylesheet_path(&self) -> PathBuf {
        self.options.output_directory.join("main.css")
    }

    // create all needed resources used by markdown (folders, stylesheets)
    fn create_resources(&self) -> Result<(), LssgError> {
        info!("Removing {:?}", self.options.output_directory);
        remove_dir_all(&self.options.output_directory)?;
        info!("Creating {:?}", self.options.output_directory);
        create_dir_all(&self.options.output_directory)?;

        let mut stylesheet = if let Some(p) = &self.options.global_stylesheet {
            let mut s = Stylesheet::new();
            s.load(p)?;
            s
        } else {
            Stylesheet::default()
        };
        for l in self.options.links.iter() {
            if let Rel::Stylesheet = l.rel {
                stylesheet.load(&l.path)?;
            }
        }
        info!(
            "Writing concatinated stylesheet {:?}",
            self.stylesheet_path()
        );
        write(self.stylesheet_path(), stylesheet.to_string())?;
        // FIXME create resources so concatenated css works
        println!("{:?}", stylesheet.resources());

        Ok(())
    }

    pub fn render(&self) -> Result<(), LssgError> {
        let mut stylesheet = if let Some(p) = &self.options.global_stylesheet {
            let mut s = Stylesheet::new();
            s.load(p)?;
            s
        } else {
            Stylesheet::default()
        };
        for l in self.options.links.iter() {
            if let Rel::Stylesheet = l.rel {
                stylesheet.load(&l.path)?;
            }
        }
        // info!(
        //     "Writing concatinated stylesheet {:?}",
        //     self.stylesheet_path()
        // );
        // write(self.stylesheet_path(), stylesheet.to_string())?;
        let mut site_map = SiteMap::from_index(self.options.index.clone())?;
        println!("{site_map}");
        site_map.add_stylesheet("main".into(), stylesheet, site_map.root())?;

        // self.create_resources()?;

        let render_options = HtmlDocumentRenderOptions {
            links: vec![HtmlLink {
                rel: renderer::Rel::Stylesheet,
                href: self.stylesheet_path().to_str().unwrap().to_owned(),
            }],
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
        self.render_recursive(
            &self.options.index,
            &self.options.output_directory,
            render_options,
        )
    }

    fn render_recursive(
        &self,
        input_markdown: &Path,
        output_directory: &Path,
        mut render_options: HtmlDocumentRenderOptions,
    ) -> Result<(), LssgError> {
        create_dir_all(output_directory)?;
        let file = File::open(input_markdown)?;
        let mut tokens = Parser::parse(file)?;

        /// Render any links to local markdown files too
        fn render_pages(
            tokens: &mut Vec<Token>,
            lssg: &Lssg,
            input_markdown: &Path,
            output_directory: &Path,
            render_options: HtmlDocumentRenderOptions,
        ) -> Result<(), LssgError> {
            tokens
                .into_iter()
                .map(|t| match t {
                    Token::Heading { tokens, .. } => render_pages(
                        tokens,
                        lssg,
                        input_markdown,
                        output_directory,
                        render_options.clone(),
                    ),
                    Token::Paragraph { tokens, .. } => render_pages(
                        tokens,
                        lssg,
                        input_markdown,
                        output_directory,
                        render_options.clone(),
                    ),
                    Token::Link { href, .. } => {
                        if href.starts_with("./") && href.ends_with(".md") {
                            let path = input_markdown.parent().unwrap().join(Path::new(&href));
                            let output = output_directory
                                .join(path.parent().unwrap().join(path.file_stem().unwrap()));
                            // remove file extension
                            href.replace_range((href.len() - 3)..href.len(), "");
                            println!("{output:?}");
                            lssg.render_recursive(&path, &output, render_options.clone())?;
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                })
                .collect()
        }

        render_pages(
            &mut tokens,
            self,
            input_markdown,
            output_directory,
            render_options.clone(),
        )?;

        let rel_path = output_directory
            .strip_prefix(&self.options.output_directory)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "failed to strip prefix"))?
            .to_string_lossy();
        let folder_depth = if rel_path.is_empty() {
            0
        } else {
            rel_path.chars().filter(|c| c == &'/').count() + 1
        };
        let new_path = "../".repeat(folder_depth) + "main.css";
        info!(
            "Updating html link {} to {}",
            render_options.links[0].href, new_path
        );
        render_options.links[0].href = new_path;

        let html = HtmlDocument::render(tokens, render_options);
        let html_output_path = output_directory.join("index.html");
        info!("Writing to {html_output_path:?}");
        write(html_output_path, html)?;

        Ok(())
    }
}
