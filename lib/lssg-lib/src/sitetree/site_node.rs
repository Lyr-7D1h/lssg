use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use crate::{LssgError, path_extension::PathExtension, sitetree::SiteId, tree::Node};
use pathdiff::diff_paths;
use reqwest::Url;

use super::stylesheet::Stylesheet;
use super::{Resource, page::Page};

/// Resolve `.` and `..` components
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            Component::ParentDir => {
                // Only pop if we have something to pop and it's not a prefix/root
                if let Some(last) = components.last()
                    && !matches!(last, Component::Prefix(_) | Component::RootDir)
                {
                    components.pop();
                }
            }
            Component::CurDir => {
                // Skip current directory references
            }
            _ => {
                components.push(component);
            }
        }
    }

    components.iter().collect()
}

/// Wrapper around absolute path to either an internal or external (http://) file
#[derive(Debug, Clone, Hash, Eq, PartialEq)] // TODO check if Hash is valid
pub enum Input {
    Local { path: PathBuf },
    External { url: Url },
}
impl Input {
    /// Create an Input from string
    pub fn from_string(string: &str) -> Result<Input, LssgError> {
        // if starts with http must be absolute
        if string.starts_with("http") {
            let url = Url::parse(string).unwrap(); // TODO always ping url to check if exists
            Ok(Input::External { url })
        } else {
            let mut path = PathBuf::from(string);
            path = fs::canonicalize(path)?;

            Ok(Input::Local { path })
        }
    }

    pub fn make_relative(&self, to: &Input) -> Option<String> {
        match self {
            Input::Local { path: from_path } => match to {
                Input::Local { path: to_path } => {
                    let from_path = if from_path.is_file() {
                        from_path.parent().unwrap_or(from_path)
                    } else {
                        from_path
                    };
                    diff_paths(to_path, from_path).and_then(|p| p.to_str().map(|s| s.to_string()))
                }
                _ => None,
            },
            Input::External { url: from_url } => match to {
                Input::External { url: to_url } => from_url.make_relative(to_url),
                _ => None,
            },
        }
    }

    /// check if string looks like a relative path
    pub fn is_relative(input: &str) -> bool {
        !input.starts_with("/") && !input.starts_with("http")
    }

    /// Create a new Input with path relative to `self` or absolute path
    pub fn new(&self, path_string: &str) -> Result<Input, LssgError> {
        // if empty just return a clone
        if path_string.is_empty() {
            return Ok(self.clone());
        }

        // return new if absolute
        if path_string.starts_with("http") {
            let url = Url::parse(path_string).map_err(|e| LssgError::parse(e.to_string()))?;
            Ok(Input::External { url })
        } else {
            match self {
                Input::Local { path } => {
                    // relative local path
                    let path: &Path = if path.filename_from_path()?.contains(".") {
                        path.parent().unwrap_or(path)
                    } else {
                        path
                    };
                    let mut path = path.join(path_string);

                    // Make path absolute if it's relative
                    if path.is_relative() {
                        path = std::env::current_dir()
                            .map_err(|e| {
                                LssgError::from(e).with_context("Failed to get current directory")
                            })?
                            .join(path);
                    }

                    // Normalize the path (resolve . and .. components)
                    path = normalize_path(&path);

                    Ok(Input::Local { path })
                }
                Input::External { url } => {
                    // relative url path
                    let url = url
                        .join(path_string)
                        .map_err(|e| LssgError::parse(e.to_string()))?;
                    Ok(Input::External { url })
                }
            }
        }
    }

    /// Check if the input exists (local file exists or external URL is reachable)
    pub fn exists(&self) -> bool {
        match self {
            Input::Local { path } => path.exists(),
            Input::External { url } => {
                // Try to make a HEAD request to check if URL is reachable
                reqwest::blocking::Client::new()
                    .head(url.clone())
                    .send()
                    .map(|response| response.status().is_success())
                    .unwrap_or(false)
            }
        }
    }

    pub fn filestem(&self) -> Result<String, LssgError> {
        match self {
            Input::Local { path } => path.filestem_from_path(),
            Input::External { url } => Path::new(url.path()).filestem_from_path(),
        }
    }
    pub fn filename(&self) -> Result<String, LssgError> {
        match self {
            Input::Local { path } => path.filename_from_path(),
            Input::External { url } => Path::new(url.path()).filename_from_path(),
        }
    }
    pub fn readable(&self) -> Result<Box<dyn Read>, LssgError> {
        match self {
            Input::Local { path } => {
                let file = File::open(path).map_err(|e| {
                    LssgError::from(e).with_context(format!("Failed to read file '{path:?}"))
                })?;
                Ok(Box::new(file))
            }
            Input::External { url } => {
                let response = reqwest::blocking::get(url.clone()).map_err(|e| {
                    LssgError::from(e).with_context(format!("Failed to get url '{url:?}"))
                })?;
                let content = Cursor::new(response.bytes().unwrap());
                Ok(Box::new(content))
            }
        }
    }
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Input::Local { path } => write!(f, "{}", path.to_string_lossy()),
            Input::External { url } => write!(f, "{}", url),
        }
    }
}

#[derive(Debug)]
pub enum SiteNodeKind {
    Stylesheet(Stylesheet),
    Page(Page),
    Resource(Resource),
    Folder,
}
impl SiteNodeKind {
    pub fn input_is_page(input: &Input) -> bool {
        input.to_string().ends_with(".md")
    }
    pub fn input_is_stylesheet(input: &Input) -> bool {
        input.to_string().ends_with(".css")
    }
    pub fn is_page(&self) -> bool {
        matches!(self, SiteNodeKind::Page { .. })
    }
}
impl std::fmt::Display for SiteNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteNodeKind::Stylesheet { .. } => write!(f, "Stylesheet"),
            SiteNodeKind::Page { .. } => write!(f, "Page"),
            SiteNodeKind::Resource { .. } => write!(f, "Resource"),
            SiteNodeKind::Folder => write!(f, "Folder"),
        }
    }
}

#[derive(Debug)]
pub struct SiteNode {
    /// Unique name within children of node
    pub name: String,
    pub parent: Option<SiteId>,
    pub children: Vec<SiteId>,
    pub kind: SiteNodeKind,
}
impl Node<SiteId> for SiteNode {
    fn children(&self) -> &Vec<SiteId> {
        &self.children
    }

    fn parent(&self) -> Option<SiteId> {
        self.parent
    }
}
impl SiteNode {
    pub fn stylesheet(name: impl Into<String>, parent: SiteId, stylesheet: Stylesheet) -> SiteNode {
        SiteNode {
            name: name.into(),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet(stylesheet),
        }
    }
    pub fn resource(name: impl Into<String>, parent: SiteId, resource: Resource) -> SiteNode {
        SiteNode {
            name: name.into(),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Resource(resource),
        }
    }
    pub fn folder(name: impl Into<String>, parent: SiteId) -> SiteNode {
        SiteNode {
            name: name.into(),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Folder,
        }
    }
    pub fn page(name: impl Into<String>, parent: SiteId, page: Page) -> SiteNode {
        SiteNode {
            name: name.into(),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Page(page),
        }
    }
}
