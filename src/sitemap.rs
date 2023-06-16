use core::fmt;
use std::{
    collections::{HashMap, HashSet},
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
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub node_type: NodeType,
}

fn filestem_from_path(path: &Path) -> Result<String, LssgError> {
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

fn filename_from_path(path: &Path) -> Result<String, LssgError> {
    Ok(path
        .file_name()
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
        let root = Self::from_index_recursive(&mut nodes, index.clone(), None)?;
        Ok(SiteMap {
            nodes,
            root,
            index_path: index,
        })
    }

    fn from_index_recursive(
        nodes: &mut Vec<Node>,
        input: PathBuf,
        parent: Option<usize>,
    ) -> Result<usize, LssgError> {
        let file = File::open(&input)?;
        let mut tokens = Parser::parse(file)?;

        // create early because of the need of an parent id
        nodes.push(Node {
            name: filestem_from_path(&input)?,
            parent,
            children: vec![],            // filling later
            node_type: NodeType::Folder, // hack changing after children created
        });
        let id = nodes.len() - 1;

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
                            let id = Self::from_index_recursive(nodes, path, Some(id))?;
                            children.push(id)
                        }
                    }
                    _ => {}
                }
            }
        }

        nodes[id].children = children;
        nodes[id].node_type = NodeType::Page { tokens, input };
        return Ok(id);
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

    // Get the path of a node
    pub fn path(&self, mut id: usize) -> String {
        if id == self.root {
            return String::new();
        }

        let mut path = vec![self.nodes[id].name.clone()];
        while let Some(i) = self.nodes[id].parent {
            if i == self.root {
                break;
            }
            path.push(self.nodes[i].name.clone());
            id = i;
        }
        path.reverse();
        path.join("/")
    }

    // Get the relative path between two nodes
    pub fn rel_path(&self, mut from: usize, mut to: usize) -> String {
        let mut visited = HashMap::new();
        let mut to_path = vec![self.nodes[to].name.clone()];

        let mut depth = 1;
        while let Some(i) = self.nodes[to].parent {
            visited.insert(i, depth);
            depth += 1;
            to = i;
            if i != self.root {
                to_path.push(self.nodes[i].name.clone())
            }
        }

        depth = 0;
        let mut to_depth = to_path.len();
        while let Some(i) = self.nodes[from].parent {
            depth += 1;
            from = i;
            if let Some(f_depth) = visited.get(&i) {
                to_depth = *f_depth;
                break;
            }
        }

        to_path.reverse();
        let path = format!("{}{}", "../".repeat(depth), to_path[0..to_depth].join("/"));

        path
    }

    pub fn add(&mut self, node: Node, parent_id: usize) -> Result<usize, LssgError> {
        // return id if already exists
        if let Some(id) = self.nodes[parent_id]
            .children
            .iter()
            .find(|n| self.nodes[**n].name == node.name)
        {
            return Ok(*id);
        }

        self.nodes.push(node);
        let id = self.nodes.len() - 1;
        self.nodes[parent_id].children.push(id);
        Ok(id)
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
                            name: filename_from_path(&resource)?,
                            parent: Some(parent_id),
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
                        name: path_part.to_owned(),
                        parent: Some(parent_id),
                        children: vec![],
                        node_type: NodeType::Folder,
                    },
                    parent_id,
                )?;
            }
        }

        self.add(
            Node {
                name,
                parent: Some(parent_id),
                children: vec![],
                node_type: NodeType::Stylesheet(stylesheet),
            },
            parent_id,
        )
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
                for _ in 0..(depth - 1) * 2 {
                    out.push('\t')
                }
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
