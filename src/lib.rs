pub mod parser;
pub mod renderer;

use std::{
    fs::{create_dir_all, write, File},
    io,
    path::{Path, PathBuf},
};

use parser::{lexer::Token, parse_error::ParseError, Parser};

use crate::renderer::{DefaultHtmlRenderer, Renderer};

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

pub struct Lssg {
    output_directory: PathBuf,
}

impl Lssg {
    pub fn new(output_directory: PathBuf) -> Lssg {
        Lssg { output_directory }
    }

    pub fn render(&self, input_markdown: &Path) -> Result<(), LssgError> {
        self.render_recursive(input_markdown, &self.output_directory)
    }

    fn render_recursive(
        &self,
        input_markdown: &Path,
        output_directory: &Path,
    ) -> Result<(), LssgError> {
        create_dir_all(output_directory)?;

        let file = File::open(input_markdown)?;
        let mut tokens = Parser::parse(file)?;

        fn render_pages(
            tokens: &mut Vec<Token>,
            lssg: &Lssg,
            input_markdown: &Path,
            output_directory: &Path,
        ) -> Result<(), LssgError> {
            tokens
                .into_iter()
                .map(|t| match t {
                    Token::Heading { tokens, .. } => {
                        render_pages(tokens, lssg, input_markdown, output_directory)
                    }
                    Token::Paragraph { tokens, .. } => {
                        render_pages(tokens, lssg, input_markdown, output_directory)
                    }
                    Token::Link { href, .. } => {
                        if href.starts_with("./") && href.ends_with(".md") {
                            let path = input_markdown.parent().unwrap().join(Path::new(&href));
                            let output = output_directory.join(path.file_stem().unwrap());
                            href.replace_range((href.len() - 3)..href.len(), "");
                            lssg.render_recursive(&path, &output)?;
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                })
                .collect()
        }

        render_pages(&mut tokens, self, input_markdown, output_directory)?;

        let html = DefaultHtmlRenderer::render(tokens);
        write(output_directory.join("index.html"), html)?;

        Ok(())
    }
}
