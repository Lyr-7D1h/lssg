mod lmarkdown;

use std::{fs::File, io, path::Path};

use lmarkdown::{parse_error::ParseError, LMarkdown};

pub enum LssgError {
    ParseError(ParseError),
    Io(io::Error),
}
impl From<io::Error> for LssgError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct Lssg {}

impl Lssg {
    pub fn new() -> Lssg {
        Lssg {}
    }

    pub fn add_index(&mut self, markdown_document: &Path) -> Result<(), LssgError> {
        let file = File::open(markdown_document)?;
        LMarkdown::parse(file);
        todo!()
    }
}
