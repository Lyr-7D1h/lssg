pub mod parser;
pub mod renderer;

use std::{
    fs::{create_dir_all, read_to_string, write, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const DEFAULT_STYLESHEET: &'static str = include_str!("default_stylesheet.css");

use log::info;
use parser::{lexer::Token, parse_error::ParseError, Parser};
use renderer::{HtmlDocument, HtmlDocumentRenderOptions, HtmlLink, Meta, Rel};

#[derive(Debug)]
pub enum LssgError {
    ParseError(ParseError),
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

#[derive(Debug)]
pub struct Link {
    pub rel: Rel,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct LssgOptions {
    pub output_directory: PathBuf,
    pub global_stylesheet: Option<PathBuf>,
    pub links: Vec<Link>,
    pub title: String,
    pub keywords: Vec<(String, String)>,
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

    pub fn render(&self, input_markdown: &Path) -> Result<(), LssgError> {
        info!("Creating {:?}", self.options.output_directory);
        create_dir_all(&self.options.output_directory)?;

        let stylesheet_dest = &self.options.output_directory.join("main.css");
        let mut stylesheet = if let Some(p) = &self.options.global_stylesheet {
            read_to_string(p)?
        } else {
            DEFAULT_STYLESHEET.to_owned()
        };
        for l in self.options.links.iter() {
            if let Rel::Stylesheet = l.rel {
                stylesheet += &read_to_string(&l.path)?;
            }
        }
        write(stylesheet_dest, stylesheet)?;

        let render_options = HtmlDocumentRenderOptions {
            links: vec![HtmlLink {
                rel: renderer::Rel::Stylesheet,
                href: "./".to_string()
                    + stylesheet_dest
                        .strip_prefix(&self.options.output_directory)
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::Other, "failed to strip prefix")
                        })?
                        .to_str()
                        .unwrap(),
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
            input_markdown,
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
                            let output = output_directory.join(path.file_stem().unwrap());
                            href.replace_range((href.len() - 3)..href.len(), "");
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
        info!("Updating {} to {}", render_options.links[0].href, new_path);
        render_options.links[0].href = new_path;

        let html = HtmlDocument::render(tokens, render_options);
        let html_output_path = output_directory.join("index.html");
        info!("Writing to {html_output_path:?}");
        write(html_output_path, html)?;

        Ok(())
    }
}
