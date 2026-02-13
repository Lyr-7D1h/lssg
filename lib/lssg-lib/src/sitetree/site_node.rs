use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use crate::{
    LssgError,
    path_extension::PathExtension,
    sitetree::{SiteId, javascript::Javascript},
    tree::Node,
};
use glob::glob;
use pathdiff::diff_paths;
use reqwest::Url;

use super::stylesheet::Stylesheet;
use super::{Resource, page::Page};

/// Wrapper around absolute path to either an internal or external (http://) file
///
/// It should always be a valid path to a resource
#[derive(Debug, Clone, Hash, Eq, PartialEq)] // TODO check if Hash is valid
pub enum Input {
    Local { path: PathBuf },
    External { url: Url },
}
impl Input {
    pub fn from_url(url: Url, client: &reqwest::blocking::Client) -> Result<Input, LssgError> {
        let res = client.head(url.clone()).send()?;
        if !res.status().is_success() {
            return Err(LssgError::request(format!(
                "Failed to do HEAD call to {url}"
            )));
        }
        Ok(Input::External { url })
    }

    /// Create an Input from string
    ///
    /// Returns multiple in case of a glob pattern
    pub fn from_string(
        string: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<Vec<Input>, LssgError> {
        // if starts with http must be absolute
        if string.starts_with("http") {
            let url = Url::parse(string).map_err(|e| LssgError::parse(e.to_string()))?;
            return Ok(vec![Input::from_url(url, client)?]);
        }

        if Self::is_glob(string) {
            let mut inputs = Vec::new();
            for entry in glob(string).map_err(|e| LssgError::parse(e.to_string()))? {
                let Ok(path) =
                    entry.inspect_err(|e| log::warn!("Failed to access file from {string}: {e}"))
                else {
                    continue;
                };
                let path = fs::canonicalize(path)?;
                inputs.push(Input::Local { path });
            }
            return Ok(inputs);
        }

        let mut path = PathBuf::from(string);
        path = fs::canonicalize(path)?;

        Ok(vec![Input::Local { path }])
    }

    /// Create a single Input from string
    ///
    /// Ignoring any glob patterns
    pub fn from_string_single(
        string: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<Input, LssgError> {
        // if starts with http must be absolute
        if string.starts_with("http") {
            let url = Url::parse(string).map_err(|e| LssgError::parse(e.to_string()))?;
            return Ok(Input::from_url(url, client)?);
        }

        let mut path = PathBuf::from(string);
        path = fs::canonicalize(path)?;

        Ok(Input::Local { path })
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

    pub fn is_local(input: &str) -> bool {
        !input.starts_with("http")
    }

    pub fn is_glob(input: &str) -> bool {
        let has_wildcards = input.contains('*') || input.contains('?');
        let has_char_class = input.contains('[') && input.contains(']');
        Self::is_local(input) && (has_wildcards || has_char_class)
    }

    /// check if string looks like a local relative path
    pub fn is_relative(input: &str) -> bool {
        !input.starts_with("/") && Self::is_local(input)
    }

    // only support relative links to markdown files for now
    // because this will allow absolute links to markdown files links to for
    // example https://github.com/Lyr-7D1h/airap/blob/master/README.md
    // will render a readme even though this might not be appropiate
    pub fn is_href_to_page(href: &str) -> bool {
        href.ends_with(".md") && Self::is_relative(href)
    }

    pub fn join_single(
        &self,
        path_string: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<Input, LssgError> {
        let mut inputs = self.join(path_string, client)?;
        match inputs.len() {
            0 => Err(LssgError::io(format!(
                "No matches found for input '{path_string}'"
            ))),
            1 => Ok(inputs.remove(0)),
            _ => Err(LssgError::io(format!(
                "Multiple matches found for input '{path_string}'"
            ))),
        }
    }

    /// Create a new Input with path relative to `self` or absolute path
    ///
    /// With glob support
    pub fn join(
        &self,
        path_string: &str,
        client: &reqwest::blocking::Client,
    ) -> Result<Vec<Input>, LssgError> {
        // if empty just return a clone
        if path_string.is_empty() {
            return Ok(vec![self.clone()]);
        }

        // return new if absolute
        if path_string.starts_with("http") {
            let url = Url::parse(path_string).map_err(|e| LssgError::parse(e.to_string()))?;
            Ok(vec![Input::from_url(url, client)?])
        } else {
            match self {
                Input::Local { path } => {
                    // relative local path
                    let path: &Path = if path.filename_from_path()?.contains(".") {
                        path.parent().unwrap_or(path)
                    } else {
                        path
                    };
                    let path = path.join(path_string);
                    let path = path
                        .to_str()
                        .ok_or(LssgError::io(format!("Invalid unicode {path_string}")))?;
                    Input::from_string(path, client)
                }
                Input::External { url } => {
                    // relative url path
                    let url = url
                        .join(path_string)
                        .map_err(|e| LssgError::parse(e.to_string()))?;
                    Ok(vec![Input::from_url(url, client)?])
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
    Javascript(Javascript),
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
    pub fn input_is_javascript(input: &Input) -> bool {
        input.to_string().ends_with(".js")
    }
    pub fn is_page(&self) -> bool {
        matches!(self, SiteNodeKind::Page { .. })
    }
}
impl std::fmt::Display for SiteNodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteNodeKind::Stylesheet { .. } => write!(f, "Stylesheet"),
            SiteNodeKind::Javascript(..) => write!(f, "Javascript"),
            SiteNodeKind::Page { .. } => write!(f, "Page"),
            SiteNodeKind::Resource { .. } => write!(f, "Resource"),
            SiteNodeKind::Folder => write!(f, "Folder"),
        }
    }
}

#[derive(Debug)]
pub struct SiteNode {
    /// Unique name within children of node
    name: String,
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
    /// Sanitize name to only include filesystem-safe characters
    /// Allows: alphanumeric, dash, underscore, dot, space
    /// Removes: path separators, control characters, and other invalid characters
    fn sanitize_name(name: &str) -> String {
        // First, extract just the filename component to prevent path traversal
        let name = Path::new(name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(name);

        // Replace invalid characters with underscores
        let sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ' ' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        // Remove leading/trailing dots and spaces
        let sanitized = sanitized.trim_matches(|c| c == '.' || c == ' ');

        // If empty after sanitization, use a default name
        if sanitized.is_empty() {
            "unnamed".to_string()
        } else {
            sanitized.to_string()
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = Self::sanitize_name(name)
    }

    pub fn stylesheet(name: impl Into<String>, parent: SiteId, stylesheet: Stylesheet) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet(stylesheet),
        }
    }
    pub fn javascript(name: impl Into<String>, parent: SiteId, javascript: Javascript) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Javascript(javascript),
        }
    }
    pub fn resource(name: impl Into<String>, parent: SiteId, resource: Resource) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Resource(resource),
        }
    }
    pub fn folder(name: impl Into<String>, parent: SiteId) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Folder,
        }
    }
    pub fn page(name: impl Into<String>, parent: SiteId, page: Page) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Page(page),
        }
    }

    /// NOTE: Should only be used by SiteTree
    pub fn root(name: impl Into<String>, page: Page) -> SiteNode {
        SiteNode {
            name: Self::sanitize_name(&name.into()),
            parent: None,
            children: vec![],
            kind: SiteNodeKind::Page(page),
        }
    }
}
