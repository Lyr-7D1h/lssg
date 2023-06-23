use core::fmt;
use std::{
    collections::HashMap,
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
// Get the relative path between two nodes
fn rel_path(nodes: &Vec<Node>, from: usize, mut to: usize) -> String {
    let mut visited = HashMap::new();
    let mut to_path = vec![nodes[to].name.clone()];

    // discover all parents from destination
    let mut depth = 0;
    while let Some(i) = nodes[to].parent {
        visited.insert(i, depth);
        depth += 1;
        to = i;
        // if not root (root doesn't have a parent) add to file directories
        if let Some(_) = nodes[i].parent {
            to_path.push(nodes[i].name.clone())
        }
    }

    // find shared parent and go back till that point
    depth = 0;
    let mut to_depth = to_path.len() - 1;
    let mut node = Some(from);
    while let Some(i) = node {
        if let Some(d) = visited.get(&i) {
            to_depth = *d;
            break;
        }
        depth += 1;
        node = nodes[i].parent;
    }

    // get remaining path
    to_path.reverse();
    return format!(
        "{}{}",
        "../".repeat(depth),
        to_path[to_path.len() - 1 - to_depth..to_path.len()].join("/")
    );
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

    /// parse a markdown file and any markdown references, updates corresponding links
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
                    Token::Html { tokens, .. } => queue.push(tokens),
                    Token::Link { href, .. } => {
                        if href.starts_with("./") && href.ends_with(".md") {
                            let path = input.parent().unwrap().join(Path::new(&href));
                            // remove file extension
                            // href.replace_range((href.len() - 3)..href.len(), "");
                            let child_id = Self::from_index_recursive(nodes, path, Some(id))?;
                            *href = rel_path(nodes, id, child_id);
                            children.push(child_id)
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
    pub fn rel_path(&self, from: usize, to: usize) -> String {
        rel_path(&self.nodes, from, to)
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
        mut stylesheet: Stylesheet,
        parent_id: usize,
    ) -> Result<usize, LssgError> {
        let root_path = self
            .index_path
            .parent()
            .unwrap_or(Path::new("/"))
            .to_owned();

        for resource in stylesheet.resources() {
            let resource = resource.clone();
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
                    let id = self.add(
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
                    stylesheet.update_resource(&resource, PathBuf::from(self.path(id)));
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
            out += &format!("({})", n);
            current_depth = depth + 1;
        }

        f.write_str(&out)?;
        Ok(())
    }
}
