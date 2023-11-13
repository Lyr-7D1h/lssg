use core::fmt;
use std::{
    collections::HashMap,
    fs::{self, File},
    io,
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
};

use log::warn;

use crate::{
    lmarkdown::{parse_lmarkdown_from_file, Token},
    path_extension::PathExtension,
    stylesheet::Stylesheet,
    tree::{Node, Tree},
    LssgError,
};

#[derive(Debug)]
pub enum SiteNodeKind {
    Stylesheet {
        stylesheet: Stylesheet,
        // A map from href paths to node ids
        links: HashMap<String, usize>,
    },
    Page {
        tokens: Vec<Token>,
        /// A map from href paths to node ids
        links: HashMap<String, usize>,
        input: PathBuf,
        // Keep the original name in the html page
        // eg. SiteNode {name: "test.html", kind: {keep_name: true}}
        // creates test.html
        // keep_name: bool,
    },
    Resource {
        input: PathBuf,
    },
    Folder,
}
impl SiteNodeKind {
    pub fn stylesheet(stylesheet: Stylesheet) -> SiteNodeKind {
        SiteNodeKind::Stylesheet {
            stylesheet,
            links: HashMap::new(),
        }
    }
    pub fn page(input: PathBuf) -> Result<SiteNodeKind, LssgError> {
        Ok(SiteNodeKind::Page {
            tokens: parse_lmarkdown_from_file(&input)?,
            links: HashMap::new(),
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
    cannonical_root_parent_path: PathBuf,
}

impl SiteTree {
    pub fn from_index(index: PathBuf) -> Result<SiteTree, LssgError> {
        let mut tree = SiteTree {
            nodes: vec![],
            root: 0,
            cannonical_root_parent_path: fs::canonicalize(&index)?
                .parent()
                .unwrap_or(Path::new("/"))
                .to_path_buf(),
        };
        let tokens = parse_lmarkdown_from_file(&index)?;
        tree.add_page(index, tokens, None)?;
        Ok(tree)
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

    /// Find a node by input path
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
        self.nodes.get(id).ok_or(LssgError::sitetree(&format!(
            "Could not find {id} in SiteTree"
        )))
    }

    /// Get all parents from a node
    pub fn parents(&self, id: usize) -> Vec<usize> {
        let mut parent = self.nodes[id].parent;
        let mut parents = vec![];
        while let Some(p) = parent {
            parents.push(p);
            parent = self.nodes[p].parent;
        }
        parents
    }

    /// Get the path of a node
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

    /// Get the relative path between two nodes
    pub fn rel_path(&self, from: usize, to: usize) -> String {
        rel_path(&self.nodes, from, to)
    }

    fn add_node(&mut self, node: SiteNode) -> usize {
        let id = self.nodes.len();
        if let Some(parent) = node.parent {
            self.nodes[parent].children.push(id);
        }
        self.nodes.push(node);
        id
    }

    pub fn add(
        &mut self,
        name: String,
        kind: SiteNodeKind,
        parent_id: usize,
    ) -> Result<usize, LssgError> {
        // return id if already exists
        if let Some(id) = self.nodes[parent_id]
            .children
            .iter()
            .find(|n| self.nodes[**n].name == name)
        {
            warn!("{} already exists SiteTree", name);
            return Ok(*id);
        }

        let id = match kind {
            SiteNodeKind::Stylesheet { stylesheet, .. } => {
                self.add_stylesheet(name, stylesheet, parent_id)?
            }
            SiteNodeKind::Page { input, tokens, .. } => {
                self.add_page(input, tokens, Some(parent_id))?
            }
            _ => self.add_node(SiteNode {
                name,
                parent: Some(parent_id),
                children: vec![],
                kind,
            }),
        };

        Ok(id)
    }

    fn add_page(
        &mut self,
        input: PathBuf,
        tokens: Vec<Token>,
        parent: Option<usize>,
    ) -> Result<usize, LssgError> {
        // create early because of the need of an parent id
        let id = self.add_node(SiteNode {
            name: input.filestem_from_path()?,
            parent,
            children: vec![],           // filling later
            kind: SiteNodeKind::Folder, // hack changing after children created
        });

        let mut children = vec![];
        let mut links = HashMap::new();
        let mut queue = vec![&tokens];
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
                            let tokens = parse_lmarkdown_from_file(&path)?;
                            let child_id = self.add_page(path, tokens, parent)?;
                            children.push(child_id);
                            links.insert(href.to_string(), child_id);
                        }
                    }
                    _ => {}
                }
            }
        }

        self[id].children = children;
        self[id].kind = SiteNodeKind::Page {
            tokens,
            links,
            input,
        };
        return Ok(id);
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    fn add_stylesheet(
        &mut self,
        name: String,
        stylesheet: Stylesheet,
        // mut links: Vec<usize>,
        parent_id: usize,
    ) -> Result<usize, LssgError> {
        for resource in stylesheet.resources() {
            let relative_to_root = resource
                .strip_prefix(&self.cannonical_root_parent_path)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "failed to strip prefix"))?;

            let mut resource_parent = parent_id.clone();
            for path_part in relative_to_root.to_str().unwrap_or("").split("/") {
                if path_part == "." {
                    continue;
                }

                // assume file when there is a file extenstion (there is a ".")
                if path_part.contains(".") {
                    let id = self.add(
                        (&resource).filename_from_path()?,
                        SiteNodeKind::Resource {
                            input: resource.clone(),
                        },
                        resource_parent,
                    )?;
                    // links.push(id);
                    break;
                }

                resource_parent =
                    self.add(path_part.to_owned(), SiteNodeKind::Folder, parent_id)?;
            }
        }

        Ok(self.add_node(SiteNode {
            name,
            parent: Some(parent_id),
            children: vec![],
            kind: SiteNodeKind::Stylesheet {
                stylesheet,
                links: HashMap::new(),
            },
        }))
    }
}

impl Tree for SiteTree {
    type Node = SiteNode;

    fn root(&self) -> usize {
        self.root
    }

    fn nodes(&self) -> &Vec<Self::Node> {
        &self.nodes
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
            out += &format!("({})({})", n, node.kind.to_string());
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
impl IndexMut<usize> for SiteTree {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}
