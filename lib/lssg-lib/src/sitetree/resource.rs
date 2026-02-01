use std::{
    fs::File,
    io::{self, Cursor, Read},
    path::Path,
};

use log::info;

use crate::lssg_error::LssgError;

use super::Input;

pub enum Resource {
    Static { content: Vec<u8> },
    Fetched { input: Input },
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource").finish()
    }
}

impl Resource {
    pub fn input(&self) -> Option<&Input> {
        match self {
            Resource::Static { .. } => None,
            Resource::Fetched { input } => Some(input),
        }
    }

    pub fn new_fetched(input: Input) -> Resource {
        Resource::Fetched { input }
    }

    pub fn from_readable(mut content: impl Read) -> Result<Resource, LssgError> {
        let mut buf = Vec::new();
        content.read_to_end(&mut buf)?;
        Ok(Resource::Static { content: buf })
    }

    pub fn new_static(content: String) -> Resource {
        Resource::Static {
            content: content.into_bytes(),
        }
    }

    pub fn readable(&self) -> Result<Box<dyn Read>, LssgError> {
        match self {
            Resource::Static { content } => Ok(Box::new(Cursor::new(content.clone()))),
            Resource::Fetched { input } => input.readable(),
        }
    }

    pub fn data(&self) -> Result<Vec<u8>, LssgError> {
        let mut readable = self.readable()?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut readable, &mut buf)?;
        Ok(buf)
    }

    pub fn write(&self, path: &Path) -> Result<(), LssgError> {
        info!("Writing resource {path:?}",);
        let mut file = File::create(path)?;
        io::copy(&mut self.readable()?, &mut file)?;
        Ok(())
    }
}
