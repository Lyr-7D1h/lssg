use std::{collections::HashMap, path::PathBuf};

use crate::{
    lmarkdown::{parse_lmarkdown_from_file, Token},
    stylesheet::Stylesheet,
    tree::Node,
    LssgError,
};

#[derive(Debug)]
pub enum SiteNodeKind {
    Stylesheet { stylesheet: Stylesheet },
    Page { tokens: Vec<Token>, input: PathBuf },
    Resource { input: PathBuf },
    Folder,
}
impl SiteNodeKind {
    pub fn stylesheet(stylesheet: Stylesheet) -> SiteNodeKind {
        SiteNodeKind::Stylesheet { stylesheet }
    }
    pub fn page(input: PathBuf) -> Result<SiteNodeKind, LssgError> {
        Ok(SiteNodeKind::Page {
            tokens: parse_lmarkdown_from_file(&input)?,
            input,
            // keep_name,
        })
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
