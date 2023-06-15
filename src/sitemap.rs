use core::fmt;
use std::{
    fs::File,
    io,
    path::{Display, Path, PathBuf},
};

use crate::{
    parser::{lexer::Token, Parser},
    stylesheet::Stylesheet,
    LssgError,
};

#[derive(Debug)]
pub enum Node {
    Stylesheet(Stylesheet),
    Page {
        children: Vec<usize>,
        tokens: Vec<Token>,
        input: PathBuf,
    },
    Resource {
        path: PathBuf,
    },
    Folder {
        childeren: Vec<usize>,
    },
}

/// Code representation of all nodes within the site (hiarchy and how nodes are related)
#[derive(Debug)]
pub struct SiteMap {
    nodes: Vec<Node>,
    root: usize,
    index_path: PathBuf,
}

impl SiteMap {
    pub fn from_index(index: PathBuf) -> Result<SiteMap, LssgError> {
        let mut nodes = vec![];
        let root = Self::from_index_recursive(&mut nodes, index.clone())?;
        Ok(SiteMap {
            nodes,
            root,
            index_path: index,
        })
    }

    fn from_index_recursive(nodes: &mut Vec<Node>, input: PathBuf) -> Result<usize, LssgError> {
        let file = File::open(&input)?;
        let mut tokens = Parser::parse(file)?;

        let mut children = vec![];
        let mut queue = vec![&mut tokens];
        while let Some(tokens) = queue.pop() {
            for t in tokens {
                match t {
                    Token::Heading { tokens, .. } => queue.push(tokens),
                    Token::Paragraph { tokens, .. } => queue.push(tokens),
                    Token::Link { href, .. } => {
                        if href.starts_with("./") && href.ends_with(".md") {
                            let path = input.parent().unwrap().join(Path::new(&href));
                            // remove file extension
                            href.replace_range((href.len() - 3)..href.len(), "");
                            let id = Self::from_index_recursive(nodes, path)?;
                            children.push(id)
                        }
                    }
                    _ => {}
                }
            }
        }

        nodes.push(Node::Page {
            children,
            tokens,
            input,
        });
        return Ok(nodes.len() - 1);
    }

    pub fn root(&self) -> usize {
        self.root
    }

    pub fn get(&self, id: usize) -> Result<&Node, LssgError> {
        self.nodes.get(id).ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Could not find {id} in SiteMap"),
        )))
    }

    pub fn add(&mut self, node: Node, parent_id: usize) -> Result<usize, LssgError> {
        self.nodes.push(node);
        Ok(self.nodes.len() - 1)
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    pub fn add_stylesheet(
        &mut self,
        stylesheet: Stylesheet,
        parent_id: usize,
    ) -> Result<usize, LssgError> {
        let root_path = self
            .index_path
            .parent()
            .unwrap_or(Path::new("/"))
            .to_owned();

        for resource in stylesheet.resources() {
            let relative = resource
                .strip_prefix(&root_path)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "failed to strip prefix"))?;

            let mut parent_id = parent_id.clone();
            for name in relative.to_str().unwrap_or("").split("/") {
                if name == "." {
                    continue;
                }
                // assume file when there is a file extenstion (there is a ".")
                if name.contains(".") {
                    self.add(
                        Node::Resource {
                            path: resource.clone(),
                        },
                        parent_id,
                    )?;
                    break;
                }

                parent_id = self.add(Node::Folder { childeren: vec![] }, parent_id)?;
            }
        }

        todo!()
    }
}

impl fmt::Display for SiteMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}
