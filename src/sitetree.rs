use core::fmt;
use std::{
    collections::HashMap,
    fs::File,
    io,
    ops::Index,
    path::{Path, PathBuf},
};

use log::warn;

use crate::{
    parser::{lexer::Token, Parser},
    stylesheet::Stylesheet,
    util::{filename_from_path, filestem_from_path},
    LssgError,
};

#[derive(Debug)]
pub enum SiteNodeKind {
    Stylesheet(Stylesheet),
    Page {
        tokens: Vec<Token>,
        input: PathBuf,
        /// Keep the original name in the html page
        keep_name: bool,
    },
    Resource {
        input: PathBuf,
    },
    Folder,
}

#[derive(Debug)]
pub struct SiteNode {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub kind: SiteNodeKind,
}

// Get the relative path between two nodes
fn rel_path(nodes: &Vec<SiteNode>, from: usize, to: usize) -> String {
    let mut visited = HashMap::new();
    let mut to_path = vec![nodes[to].name.clone()];

    // discover all parents from destination
    let mut depth = 0;
    let mut node = nodes[to].parent;
    while let Some(i) = node {
        visited.insert(i, depth);
        depth += 1;
        node = nodes[i].parent;
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

    // don't add anything to path traversal if root
    to_path.reverse();
    let to_path = if nodes[to].parent.is_some() {
        to_path[to_path.len() - 1 - to_depth..to_path.len()].join("/")
    } else {
        depth -= 1;
        "".into()
    };

    // get remaining path
    return format!("{}{}", "../".repeat(depth), to_path);
}

/// Code representation of all nodes within the site (hiarchy and how nodes are related)
#[derive(Debug)]
pub struct SiteTree {
    nodes: Vec<SiteNode>,
    root: usize,
    index_path: PathBuf,
}

impl SiteTree {
    pub fn from_index(index: PathBuf) -> Result<SiteTree, LssgError> {
        let mut nodes = vec![];
        let root = Self::from_index_recursive(&mut nodes, index.clone(), None)?;
        Ok(SiteTree {
            nodes,
            root,
            index_path: index,
        })
    }

    /// parse a markdown file and any markdown references, updates corresponding links
    fn from_index_recursive(
        nodes: &mut Vec<SiteNode>,
        input: PathBuf,
        parent: Option<usize>,
    ) -> Result<usize, LssgError> {
        let file = File::open(&input)?;
        let mut tokens = Parser::parse(file)?;

        // create early because of the need of an parent id
        nodes.push(SiteNode {
            name: filestem_from_path(&input)?,
            parent,
            children: vec![],           // filling later
            kind: SiteNodeKind::Folder, // hack changing after children created
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
        nodes[id].kind = SiteNodeKind::Page {
            tokens,
            input,
            keep_name: false,
        };
        return Ok(id);
    }

    /// Check if node `id` has `parent_id` as parent node
    pub fn is_parent(&self, id: usize, parent_id: usize) -> bool {
        let mut parent = self.nodes[id].parent;
        while let Some(p) = parent {
            if p == parent_id {
                return true;
            }
            parent = self.nodes[id].parent
        }
        return false;
    }

    pub fn find_by_input(&self, finput: &Path) -> Option<usize> {
        let mut queue = vec![self.root];
        while let Some(id) = queue.pop() {
            let node = &self.nodes[id];
            match &node.kind {
                SiteNodeKind::Page { input, .. } => {
                    if input == finput {
                        return Some(id);
                    }
                }
                SiteNodeKind::Resource { input, .. } => {
                    if input == finput {
                        return Some(id);
                    }
                }
                _ => {}
            }
            queue.append(&mut node.children.clone())
        }

        None
    }

    pub fn root(&self) -> usize {
        self.root
    }

    pub fn get(&self, id: usize) -> Result<&SiteNode, LssgError> {
        self.nodes.get(id).ok_or(LssgError::sitemap(&format!(
            "Could not find {id} in SiteMap"
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

    pub fn add(&mut self, node: SiteNode, parent_id: usize) -> Result<usize, LssgError> {
        // return id if already exists
        if let Some(id) = self.nodes[parent_id]
            .children
            .iter()
            .find(|n| self.nodes[**n].name == node.name)
        {
            warn!("{} already exists SiteTree", node.name);
            return Ok(*id);
        }

        // FIXME check if page and then update links, find id by input

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
                        SiteNode {
                            name: filename_from_path(&resource)?,
                            parent: Some(parent_id),
                            children: vec![],
                            kind: SiteNodeKind::Resource {
                                input: resource.clone(),
                            },
                        },
                        parent_id,
                    )?;
                    stylesheet.update_resource(&resource, PathBuf::from(self.path(id)));
                    break;
                }

                parent_id = self.add(
                    SiteNode {
                        name: path_part.to_owned(),
                        parent: Some(parent_id),
                        children: vec![],
                        kind: SiteNodeKind::Folder,
                    },
                    parent_id,
                )?;
            }
        }

        self.add(
            SiteNode {
                name,
                parent: Some(parent_id),
                children: vec![],
                kind: SiteNodeKind::Stylesheet(stylesheet),
            },
            parent_id,
        )
    }
}

impl fmt::Display for SiteTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out: String = String::new();

        // TODO use BFS struct
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

impl Index<usize> for SiteTree {
    type Output = SiteNode;

    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}
