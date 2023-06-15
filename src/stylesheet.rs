use std::{
    collections::HashSet,
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
    resources: HashSet<PathBuf>,
}

pub fn urls(content: &String) -> Result<Vec<String>, LssgError> {
    let re = Regex::new(r#"url\("(\.[^)"]*)"\)"#)?;
    Ok(re.captures_iter(content).map(|m| m[1].to_owned()).collect())
}

impl Stylesheet {
    /// Create new stylesheet with default
    pub fn default() -> Stylesheet {
        Stylesheet {
            content: DEFAULT_STYLESHEET.to_owned(),
            resources: HashSet::new(),
        }
    }

    /// create new empty stylesheet
    pub fn new() -> Stylesheet {
        Stylesheet {
            content: String::new(),
            resources: HashSet::new(),
        }
    }

    /// Load stylesheet and discover local referenced resources
    pub fn load(&mut self, path: &Path) -> Result<(), LssgError> {
        let content = read_to_string(path)?;

        for r in urls(&content)?.into_iter() {
            self.resources
                .insert(path.parent().unwrap_or(Path::new("/")).join(r));
        }
        self.content += &content;
        Ok(())
    }

    pub fn resources(&self) -> &HashSet<PathBuf> {
        &self.resources
    }
}

impl ToString for Stylesheet {
    fn to_string(&self) -> String {
        self.content.clone()
    }
}
