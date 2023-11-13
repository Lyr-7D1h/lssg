use std::{
    collections::{HashMap, HashSet},
    fs::{self, read_to_string},
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::LssgError;

const DEFAULT_STYLESHEET: &'static str = include_str!("default_stylesheet.css");

/// Stylesheet representation for resource discovering and condensing multiple stylesheets into one
#[derive(Debug, Clone)]
pub struct Stylesheet {
    content: String,
    /// Map of raw resource strings inside of content indexed by cannonical path to resource
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

    /// Add resource
    fn add_resource(&mut self, input: PathBuf, raw: String) {
        if let Some(set) = self.resources.get_mut(&input) {
            set.insert(raw);
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

        let parent_path = path.parent().unwrap_or(Path::new("/"));

        let re = Regex::new(r#"url\("?(\.[^)"]*)"?\)"#)?;
        for r in re.captures_iter(&content).into_iter() {
            let input = fs::canonicalize(parent_path.join(&r[1]))?;
            self.add_resource(input, r[0].to_owned());
        }
        self.content += &content;
        Ok(())
    }

    /// Get all cannonicalized path to resources in stylesheet
    pub fn resources(&self) -> Vec<&PathBuf> {
        Vec::from_iter(self.resources.keys())
    }

    /// Update a resource input path to a new one
    pub fn update_resource(&mut self, resource: &Path, updated_resource: String) -> bool {
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

    pub fn to_string(self) -> String {
        self.content
    }
}
