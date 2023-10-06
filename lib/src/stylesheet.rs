use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::LssgError;

const DEFAULT_STYLESHEET: &'static str = include_str!("default_stylesheet.css");

/// Stylesheet representation for resource discovering and condensing multiple stylesheets into one
#[derive(Debug)]
pub struct Stylesheet {
    content: String,
    resources: HashMap<PathBuf, HashSet<String>>,
}

impl Stylesheet {
    /// Create new stylesheet with default
    pub fn default() -> Stylesheet {
        Stylesheet {
            content: DEFAULT_STYLESHEET.to_owned(),
            resources: HashMap::new(),
        }
    }

    /// create new empty stylesheet
    pub fn new() -> Stylesheet {
        Stylesheet {
            content: String::new(),
            resources: HashMap::new(),
        }
    }

    pub fn add_resource(&mut self, input: PathBuf, raw: String) {
        if let Some(r) = self.resources.get_mut(&input) {
            r.insert(raw);
        } else {
            let mut set = HashSet::new();
            set.insert(raw);
            self.resources.insert(input, set);
        }
    }

    /// Append stylesheet and discover local referenced resources
    pub fn append(&mut self, path: &Path) -> Result<(), LssgError> {
        let content = read_to_string(path)
            .map_err(|_| LssgError::io(&format!("Failed to append stylesheet: {path:?}")))?;

        let re = Regex::new(r#"url\("?(\.[^)"]*)"?\)"#)?;
        for r in re.captures_iter(&content).into_iter() {
            self.add_resource(
                path.parent().unwrap_or(Path::new("/")).join(&r[1]),
                r[0].to_owned(),
            );
        }
        self.content += &content;
        Ok(())
    }

    pub fn resources(&self) -> Vec<PathBuf> {
        Vec::from_iter(self.resources.keys().map(|k| k.clone()))
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, resource: &Path, updated_resource: PathBuf) -> bool {
        let updated_resource = updated_resource.to_string_lossy().to_string();
        match self.resources.get_mut(resource) {
            None => false,
            Some(raw) => {
                for r in raw.iter() {
                    self.content = self
                        .content
                        .replace(r, &format!("url(\"{}\")", updated_resource));
                }
                let mut new_raw = HashSet::new();
                new_raw.insert(updated_resource);
                *raw = new_raw;

                true
            }
        }
    }
}

impl ToString for Stylesheet {
    fn to_string(&self) -> String {
        self.content.clone()
    }
}
