use core::fmt;
use std::{
    collections::HashMap,
    fs::{self},
    io,
    ops::{Index, IndexMut},
    path::{Path, PathBuf},
};

use log::{debug, warn};

use crate::{
    lmarkdown::{parse_lmarkdown_from_file, Token},
    path_extension::PathExtension,
    stylesheet::Stylesheet,
    tree::Tree,
    LssgError,
};

use super::{
    relational_graph::RelationalGraph,
    relational_graph::{Link, Relation},
    SiteNode, SiteNodeKind,
};

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
    if depth > 0 {
        return format!("{}{}", "../".repeat(depth), to_path);
    } else {
        return format!("./{}", to_path);
    }
}

/// Code representation of all nodes within the site (hiarchy and how nodes are related)
#[derive(Debug)]
pub struct SiteTree {
    nodes: Vec<SiteNode>,
    root: usize,
    // TODO make non pub when css minification is done
    pub cannonical_root_parent_path: PathBuf,

    rel_graph: RelationalGraph,
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
            rel_graph: RelationalGraph::new(),
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
        // TODO use files_to_ids
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

    // get a node by name by checking the children of `id`
    pub fn get_by_name(&self, name: &str, id: usize) -> Option<&usize> {
        self.nodes[id]
            .children
            .iter()
            .find(|n| &self.nodes[**n].name == name)
    }

    pub fn get(&self, id: usize) -> Result<&SiteNode, LssgError> {
        self.nodes.get(id).ok_or(LssgError::sitetree(&format!(
            "Could not find {id} in SiteTree"
        )))
    }

    /// get next parent of page
    pub fn page_parent(&self, id: usize) -> Option<usize> {
        let mut parent = self.nodes[id].parent;
        let mut parents = vec![];
        while let Some(p) = parent {
            if let SiteNodeKind::Page { .. } = self.nodes[p].kind {
                return Some(p);
            }
            parents.push(p);
            parent = self.nodes[p].parent;
        }
        None
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

    pub fn ids(&self) -> Vec<usize> {
        (0..self.nodes.len() - 1).collect()
    }

    /// Get the relative path between two nodes
    pub fn rel_path(&self, from: usize, to: usize) -> String {
        rel_path(&self.nodes, from, to)
    }

    pub fn add_link(&mut self, from: usize, to: usize) {
        self.rel_graph.add(from, to, Relation::External);
    }

    /// Get all the relations from a single node to other nodes
    pub fn links_from(&self, from: usize) -> Vec<&Link> {
        self.rel_graph.links_from(from)
    }

    /// Utility function to add a node, create a id and add to parent children
    fn add_node(&mut self, node: SiteNode) -> usize {
        if let Some(parent) = node.parent {
            if let Some(id) = self.get_by_name(&node.name, parent) {
                warn!("{} already exists at {id}", node.name);
            }
        }
        let id = self.nodes.len();
        if let Some(parent) = node.parent {
            self.nodes[parent].children.push(id);
            self.rel_graph.add(parent, id, Relation::Family);
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
        if let Some(id) = self.get_by_name(&name, parent_id) {
            // warn!("{} already exists using existing node instead", name);
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

    /// Add a page node to tree and discover any other new pages
    fn add_page(
        &mut self,
        input: PathBuf,
        tokens: Vec<Token>,
        mut parent: Option<usize>,
    ) -> Result<usize, LssgError> {
        let name = input.filestem_from_path()?;

        // update parent to folder or create folders if applicable
        if let Some(parent) = &mut parent {
            *parent = self.create_folders(&input, *parent)?;
        }

        if let Some(parent) = parent {
            if let Some(id) = self.get_by_name(&name, parent) {
                return Ok(*id);
            }
        }

        // create early because of the need of an parent id
        let id = self.add_node(SiteNode {
            name,
            parent,
            children: vec![],
            kind: SiteNodeKind::Folder, // hack changing after children created
        });

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
                            let tokens = parse_lmarkdown_from_file(&path)?;
                            let child_id = self.add_page(path, tokens, Some(id))?;
                            self.rel_graph.add(
                                id,
                                child_id,
                                Relation::Discovered {
                                    path: href.to_string(),
                                },
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        self[id].kind = SiteNodeKind::Page { tokens, input };
        return Ok(id);
    }

    /// Creates folders if needed returns the new or old parent
    fn create_folders(&mut self, input: &Path, mut parent: usize) -> Result<usize, LssgError> {
        let input = fs::canonicalize(&input)?;
        let rel_path = input
            .strip_prefix(&self.cannonical_root_parent_path)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "failed to strip prefix"))?;
        let parts: Vec<&str> = rel_path.to_str().unwrap_or("").split("/").collect();
        let parts = &parts[0..parts.len() - 1];

        let mut parents = self.parents(parent);
        parents.push(parent);
        parents.reverse();
        for i in 0..parts.len() {
            let name = parts[i];
            if let Some(parent) = parents.get(i) {
                if self[*parent].name == name {
                    continue;
                }
            }
            if let Some(id) = self.get_by_name(name, parent) {
                parent = *id;
            } else {
                debug!("creating folder {name:?} under {parent:?}");
                parent = self.add(name.to_string(), SiteNodeKind::Folder, parent)?;
            }
        }

        return Ok(parent);
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    fn add_stylesheet(
        &mut self,
        name: String,
        stylesheet: Stylesheet,
        // mut links: Vec<usize>,
        parent_id: usize,
    ) -> Result<usize, LssgError> {
        let parent_id = self.create_folders(
            stylesheet
                .input
                .as_ref()
                .expect("every stylehsheet needs an input for now"),
            parent_id,
        )?;
        if let Some(id) = self.get_by_name(&name, parent_id) {
            return Ok(*id);
        }
        let resources: Vec<PathBuf> = stylesheet
            .resources()
            .into_iter()
            .map(|p| p.clone())
            .collect();
        let stylesheet_id = self.add_node(SiteNode {
            name,
            parent: Some(parent_id),
            children: vec![],
            kind: SiteNodeKind::Stylesheet { stylesheet },
        });

        for resource in resources {
            let parent_id = self.create_folders(&resource, parent_id)?;
            let resource_id = self.add(
                resource.filename_from_path()?,
                SiteNodeKind::Resource {
                    input: resource.clone(),
                },
                parent_id,
            )?;
            self.rel_graph.add(
                stylesheet_id,
                resource_id,
                Relation::Discovered {
                    path: resource.to_string_lossy().to_string(),
                },
            );
        }

        Ok(stylesheet_id)
    }

    pub fn remove(&mut self, id: usize) {
        self.rel_graph.remove_all(id);
        todo!("remove from tree");
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
        // fill in table
        let mut row_length = 0;
        let mut table: Vec<Vec<Option<String>>> = vec![];
        let mut current_col = 0;
        let mut queue = vec![(self.root(), 0)];
        while let Some((n, col)) = queue.pop() {
            let node = &self.nodes[n];
            for c in &node.children {
                queue.push((c.clone(), col + 1))
            }

            // create col if not exists
            if let None = table.get(col) {
                table.push(vec![]);
            }

            // fill in until we reach the current row where we are
            let amount_rows_in_col = table[col].len();
            for _ in amount_rows_in_col + 1..row_length {
                table[col].push(None);
            }

            if let Some(None) = table[col].last() {
                if current_col > col {
                    table[col].push(None);
                }
            }
            current_col = col;

            let node_name = format!("{}({})({})", node.name, n, node.kind.to_string());
            table[col].push(Some(node_name));

            let amount_rows_in_col = table[col].len();
            // update at what row we are
            if amount_rows_in_col > row_length {
                row_length = amount_rows_in_col;
            }
        }

        // display table
        let mut out = vec![String::new(); row_length];
        for col in 0..table.len() {
            let max_name_length = table[col]
                .iter()
                .map(|c| c.as_ref().map(|c| c.len()).unwrap_or(0))
                .reduce(|a, b| a.max(b))
                .unwrap_or(0);
            for (row, entry) in table[col].iter().enumerate() {
                match entry {
                    Some(name) => {
                        out[row] += name;
                        out[row] += &" ".repeat(max_name_length - name.len());
                        if let Some(next_column) = table.get(col + 1) {
                            if let Some(Some(_)) = next_column.get(row) {
                                out[row] += &" - ";
                                continue;
                            }
                        }
                        out[row] += &"   ";
                    }
                    None => out[row] += &" ".repeat(max_name_length + 3),
                }
            }
            for row in table[col].len()..row_length {
                out[row] += &" ".repeat(max_name_length + 3);
            }
        }

        f.write_str(&out.join("\n"))?;
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
