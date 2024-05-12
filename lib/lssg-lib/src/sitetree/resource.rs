use std::{
    fs::File,
    io::{self, Cursor, Read},
    path::Path,
};

use log::info;

use crate::lssg_error::LssgError;

use super::Input;

pub enum Resource {
    Static { content: String },
    Fetched { input: Input },
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource").finish()
    }
}

impl Resource {
    pub fn new_fetched(input: Input) -> Result<Resource, LssgError> {
        Ok(Resource::Fetched { input })
    }

    pub fn new_static(content: String) -> Resource {
        Resource::Static { content }
    }

    pub fn readable(&self) -> Result<Box<dyn Read>, LssgError> {
        match self {
            Resource::Static { content } => Ok(Box::new(Cursor::new(content.clone().into_bytes()))),
            Resource::Fetched { input } => input.readable(),
        }
    }

    pub fn write(&mut self, path: &Path) -> Result<(), LssgError> {
        info!("Writing resource {path:?}",);
        let mut file = File::create(path)?;
        io::copy(&mut self.readable()?, &mut file)?;
        Ok(())
    }
}
