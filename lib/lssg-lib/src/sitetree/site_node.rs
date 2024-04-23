use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use crate::{path_extension::PathExtension, tree::Node, LssgError};
use pathdiff::diff_paths;
use reqwest::Url;

use super::stylesheet::Stylesheet;
use super::{page::Page, Resource};

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
            let url = Url::parse(&string).unwrap(); // TODO always ping url to check if exists
            return Ok(Input::External { url });
        }

        let mut path = PathBuf::from(&string);
        path = fs::canonicalize(path)?;

        Ok(Input::Local { path })
    }

    pub fn make_relative(&self, to: &Input) -> Option<String> {
        match self {
            Input::Local { path: from_path } => match to {
                Input::Local { path: to_path } => {
                    let from_path = if from_path.is_file() {
                        from_path.parent().unwrap_or(&from_path)
                    } else {
                        from_path
                    };
                    return diff_paths(to_path, from_path)
                        .map(|p| p.to_str().map(|s| s.to_string()))
                        .flatten();
                }
                _ => return None,
            },
            Input::External { url: from_url } => match to {
                Input::External { url: to_url } => from_url.make_relative(to_url),
                _ => return None,
            },
        }
    }

    /// check if path is a relative path
    pub fn is_relative(path: &str) -> bool {
        if path.starts_with("/") || path.starts_with("http") {
            return false;
        }
        return true;
    }

    /// Create a new Input with path relative to `self` or absolute path
    pub fn new(&self, path_string: &str) -> Result<Input, LssgError> {
        // return new if absolute
        if path_string.starts_with("http") {
            let url = Url::parse(&path_string).unwrap();
            return Ok(Input::External { url });
        }

        match self {
            Input::Local { path } => {
                // relative local path
                let path: &Path = if path.filename_from_path()?.contains(".") {
                    &path.parent().unwrap_or(&path)
                } else {
                    &path
                };
                let mut path = path.join(path_string);
                path = fs::canonicalize(path)?;
                return Ok(Input::Local { path });
            }
            Input::External { url } => {
                // relative url path
                let url = url.join(path_string).unwrap(); // TODO check if cannonical
                return Ok(Input::External { url });
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
    pub fn to_string(&self) -> String {
        match self {
            Input::Local { path } => path.to_string_lossy().to_string(),
            Input::External { url } => url.to_string(),
        }
    }
    pub fn readable(&self) -> Result<Box<dyn Read>, LssgError> {
        match self {
            Input::Local { path } => {
                let file = File::open(path)?;
                Ok(Box::new(file))
            }
            Input::External { url } => {
                // FIXME unwrap
                let response = reqwest::blocking::get(url.clone()).unwrap();
                let content = Cursor::new(response.bytes().unwrap());
                Ok(Box::new(content))
            }
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
        if let SiteNodeKind::Page { .. } = self {
            true
        } else {
            false
        }
    }
}
impl ToString for SiteNodeKind {
    fn to_string(&self) -> String {
        match self {
            SiteNodeKind::Stylesheet { .. } => "Stylesheet",
            SiteNodeKind::Page { .. } => "Page",
            SiteNodeKind::Resource { .. } => "Resource",
            SiteNodeKind::Folder => "Folder",
        }
        .into()
    }
}

#[derive(Debug)]
pub struct SiteNode {
    /// Unique name within children of node
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kind: SiteNodeKind,
}
impl Node for SiteNode {
    fn children(&self) -> &Vec<usize> {
        &self.children
    }
}
impl SiteNode {
    pub fn stylesheet(name: impl Into<String>, parent: usize, stylesheet: Stylesheet) -> SiteNode {
        SiteNode {
            name: name.into(),
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet(stylesheet),
        }
    }
}
