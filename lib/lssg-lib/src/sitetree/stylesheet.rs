use std::path::Path;
use std::{fs::write, io::Read};

use log::info;
use regex::Regex;

use crate::{sitetree::Input, LssgError};

/// Stylesheet representation for resource discovering and condensing multiple stylesheets into one
#[derive(Debug, Clone)]
pub struct Stylesheet {
    content: String,
}

impl Stylesheet {
    pub fn from_readable(mut readable: impl Read) -> Result<Stylesheet, LssgError> {
        let mut content = String::new();
        readable.read_to_string(&mut content)?;
        Ok(Stylesheet { content })
    }

    pub fn resources(&self) -> Vec<String> {
        let mut resources = vec![];
        // TODO add `@import` support
        let re = Regex::new(r#"url\("?(\.[^)"]*)"?\)"#).unwrap();
        for r in re.captures_iter(&self.content).into_iter() {
            resources.push(r[1].to_string());
        }
        return resources;
    }

    /// Append stylesheet and discover local referenced resources
    pub fn append(&mut self, _stylesheet: Stylesheet) -> Result<(), LssgError> {
        todo!()
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, raw_path: &str, updated_path: &str) {
        self.content = self.content.replace(raw_path, updated_path);
    }

    pub fn write(&mut self, path: &Path) -> Result<(), LssgError> {
        info!("Writing stylesheet {path:?}",);
        write(path, &mut self.content)?;
        Ok(())
    }
}

impl TryFrom<&Input> for Stylesheet {
    type Error = LssgError;

    fn try_from(value: &Input) -> Result<Self, Self::Error> {
        Self::from_readable(value.readable()?)
    }
}
