use std::{
    fs::File,
    io::{self, Read},
    path::Path,
};

use log::info;

use crate::lssg_error::LssgError;

use super::Input;

pub struct Resource {
    readable: Box<dyn Read>,
}

impl std::fmt::Debug for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Resource").finish()
    }
}

impl Resource {
    pub fn from_input(input: &Input) -> Result<Resource, LssgError> {
        Ok(Resource {
            readable: Box::new(input.readable()?),
        })
    }

    pub fn write(&mut self, path: &Path) -> Result<(), LssgError> {
        info!("Writing resource {path:?}",);
        let mut file = File::create(path)?;
        io::copy(&mut self.readable, &mut file)?;
        Ok(())
    }
}
