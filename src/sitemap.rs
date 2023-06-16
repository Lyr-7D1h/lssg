use core::fmt;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

use crate::{
    parser::{lexer::Token, Parser},
    stylesheet::Stylesheet,
    LssgError,
};

#[derive(Debug)]
pub enum NodeType {
    Stylesheet(Stylesheet),
    Page { tokens: Vec<Token>, input: PathBuf },
    Resource { input: PathBuf },
    Folder,
}

#[derive(Debug)]
pub struct Node {
    name: String,
    children: Vec<usize>,
    node_type: NodeType,
}

impl Node {
    pub fn new(name: String, children: Vec<usize>, node_type: NodeType) -> Node {
        Node {
            name,
            children,
            node_type,
        }
    }
}

fn name_from_path(path: &Path) -> Result<String, LssgError> {
    Ok(path
        .file_stem()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} does not have a filename"),
        )))?
        .to_str()
        .ok_or(LssgError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{path:?} is non unicode"),
        )))?
        .to_owned())
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

        nodes.push(Node {
            name: name_from_path(&input)?,
            children,
            node_type: NodeType::Page { tokens, input },
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
        name: String,
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
            for path_part in relative.to_str().unwrap_or("").split("/") {
                if path_part == "." {
                    continue;
                }
                // assume file when there is a file extenstion (there is a ".")
                if path_part.contains(".") {
                    self.add(
                        Node {
                            name: name_from_path(&resource)?,
                            children: vec![],
                            node_type: NodeType::Resource {
                                input: resource.clone(),
                            },
                        },
                        parent_id,
                    )?;
                    break;
                }

                parent_id = self.add(
                    Node {
                        name: name.clone(),
                        children: vec![],
                        node_type: NodeType::Folder,
                    },
                    parent_id,
                )?;
            }
        }

        todo!()
    }
}

impl fmt::Display for SiteMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out: String = String::new();

        let mut current_depth = 0;
        let mut queue = vec![(self.root, 0)];
        while let Some((n, depth)) = queue.pop() {
            let node = &self.nodes[n];
            for c in &node.children {
                queue.push((c.clone(), depth + 1))
            }
            if depth < current_depth {
                out.push('\n');
            }
            if current_depth != 0 {
                out += "\t - \t"
            }
            out += &node.name;
            current_depth = depth + 1;
        }

        f.write_str(&out)?;
        Ok(())
    }
}
