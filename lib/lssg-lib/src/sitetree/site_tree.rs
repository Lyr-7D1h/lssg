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
    lmarkdown::{parse_lmarkdown, parse_lmarkdown_from_file, Token},
    path_extension::PathExtension,
    tree::Tree,
    LssgError,
};

use super::{
    page::Page,
    relational_graph::RelationalGraph,
    relational_graph::{Link, Relation},
    stylesheet::Stylesheet,
    Input, SiteNode, SiteNodeKind,
};

fn absolute_path(nodes: &Vec<SiteNode>, to: usize) -> String {
    let mut names = vec![nodes[to].name.clone()];
    let mut parent = nodes[to].parent;
    while let Some(p) = parent {
        names.push(nodes[p].name.clone());
        parent = nodes[p].parent;
    }
    names.pop(); // pop root
    names.reverse();
    return format!("/{}", names.join("/"));
}

/// Get the relative path between two nodes
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
    // used for detecting if inputs are outside of the root input file
    root_input: Input,

    /// cannonical paths to node ids
    input_to_id: HashMap<Input, usize>,
    rel_graph: RelationalGraph,
}

impl SiteTree {
    /// `input` is a markdown input file from where to start discovering resources and pages
    pub fn from_input(input: Input) -> Result<SiteTree, LssgError> {
        let mut tree = SiteTree {
            nodes: vec![],
            root: 0,
            root_input: input.clone(),
            input_to_id: HashMap::new(),
            rel_graph: RelationalGraph::new(),
        };
        tree.add_page_with_root(input, None)?;
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

    // get a node by name by checking the children of `id`
    pub fn get_by_name(&self, name: &str, id: usize) -> Option<&usize> {
        self.nodes[id]
            .children
            .iter()
            .find(|n| &self.nodes[**n].name == name)
    }

    pub fn root(&self) -> usize {
        self.root
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

    /// Get the absolute path of a node
    pub fn path(&self, id: usize) -> String {
        absolute_path(&self.nodes, id)
    }

    /// Get the relative path between two nodes
    pub fn rel_path(&self, from: usize, to: usize) -> String {
        rel_path(&self.nodes, from, to)
    }

    pub fn ids(&self) -> Vec<usize> {
        (0..self.nodes.len() - 1).collect()
    }

    pub fn add_link(&mut self, from: usize, to: usize) {
        self.rel_graph.add(from, to, Relation::External);
    }

    /// Get all the relations from a single node to other nodes
    pub fn links_from(&self, from: usize) -> Vec<&Link> {
        self.rel_graph.links_from(from)
    }

    /// Utility function to add a node, create a id and add to parent children
    fn add_node(&mut self, mut node: SiteNode) -> Result<usize, LssgError> {
        // check for name collisions
        if let Some(parent) = node.parent {
            if let Some(id) = self.get_by_name(&node.name, parent) {
                warn!("{} already exists at {id}", node.name);
            }
        }

        let id = self.nodes.len();

        // register input to files to ids
        match &mut node.kind {
            SiteNodeKind::Stylesheet { input, .. }
            | SiteNodeKind::Page { input, .. }
            | SiteNodeKind::Resource { input } => {
                if let Some(id) = self.input_to_id.get(&input) {
                    warn!("{input:?} already added under {id}");
                    return Ok(*id);
                }
                self.input_to_id.insert(input.clone(), id);
            }
            _ => {}
        };

        if let Some(parent) = node.parent {
            self.nodes[parent].children.push(id);
            self.rel_graph.add(parent, id, Relation::Family);
        }
        self.nodes.push(node);

        Ok(id)
    }

    /// add from Input, will figure out what node to add from input
    pub fn add(&mut self, input: Input, parent_id: usize) -> Result<usize, LssgError> {
        // return id if already exists
        if let Some(id) = self.input_to_id.get(&input) {
            // warn!("{} already exists using existing node instead", name);
            return Ok(*id);
        }

        if SiteNodeKind::is_stylesheet(&input) {
            return self.add_stylesheet(input, parent_id);
        }
        if SiteNodeKind::is_stylesheet(&input) {
            return self.add_page(input, parent_id);
        }

        return self.add_node(SiteNode {
            name: input.filename()?,
            parent: Some(parent_id),
            children: vec![],
            kind: SiteNodeKind::Resource { input },
        });
    }

    /// Add a page node to tree and discover any other new pages
    /// will error if input is not a markdown file
    pub fn add_page(&mut self, input: Input, parent: usize) -> Result<usize, LssgError> {
        self.add_page_with_root(input, Some(parent))
    }

    /// Add a page node to tree and discover any other new pages with possibility of adding root
    fn add_page_with_root(
        &mut self,
        input: Input,
        mut parent: Option<usize>,
    ) -> Result<usize, LssgError> {
        if let Some(id) = self.input_to_id.get(&input) {
            // TODO check if needs to move pointing input
            return Ok(*id);
        }

        if let Some(parent) = &mut parent {
            *parent = self.create_folders(&input, *parent)?;
        }

        // create early because of the need of an parent id
        let page = Page::from_input(&input)?;
        let id = self.add_node(SiteNode {
            name: input.filestem()?,
            parent,
            children: vec![],
            kind: SiteNodeKind::Page {
                page,
                input: input.clone(),
            },
        })?;

        let links: Vec<(String, String)> = match &self.nodes[id].kind {
            SiteNodeKind::Page { page, .. } => page
                .links()
                .into_iter()
                .map(|(text, href)| (text.clone(), href.clone()))
                .collect(),
            _ => panic!("has to be page"),
        };

        for (text, href) in links {
            // if link has no text add whatever is in it
            if text.len() == 0 {
                let input = input.new(&href)?;
                let child_id = self.add(input, id)?;
                self.rel_graph
                    .add(id, child_id, Relation::Discovered { raw_path: href });
                continue;
            }

            // only support relative links to markdown files for now
            // because this will allow absolute links to markdown files links to for
            // example https://github.com/Lyr-7D1h/airap/blob/master/README.md
            // will render a readme even though this might not be appropiate
            if href.ends_with(".md") {
                if Input::is_relative(&href) {
                    println!("{href}");
                    let input = input.new(&href)?;
                    let child_id = self.add_page(input, id)?;
                    self.rel_graph
                        .add(id, child_id, Relation::Discovered { raw_path: href });
                    continue;
                }
            }
        }

        return Ok(id);
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    pub fn add_stylesheet(&mut self, input: Input, mut parent: usize) -> Result<usize, LssgError> {
        parent = self.create_folders(&input, parent)?;

        let stylesheet = Stylesheet::from_input(&input)?;
        let resources: Vec<String> = stylesheet
            .resources()?
            .into_iter()
            .map(|p| p.to_string())
            .collect();

        let stylesheet_id = self.add_node(SiteNode {
            name: input.filename()?,
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet {
                stylesheet,
                input: input.clone(),
            },
        })?;

        for resource in resources {
            let input = input.new(&resource)?;
            let parent = self.create_folders(&input, parent)?;
            let resource_id = self.add_node(SiteNode {
                name: input.filename()?,
                parent: Some(parent),
                children: vec![],
                kind: SiteNodeKind::Resource { input },
            })?;
            self.rel_graph.add(
                stylesheet_id,
                resource_id,
                Relation::Discovered { raw_path: resource },
            );
        }

        Ok(stylesheet_id)
    }

    /// If local input and not outside of `root_input` it will create some extra folders for
    /// structuring SiteTree
    fn create_folders(&mut self, input: &Input, mut parent: usize) -> Result<usize, LssgError> {
        if let Some(rel_path) = self.root_input.make_relative(input) {
            // don't allow backtrack from root
            if rel_path.starts_with("..") {
                return Ok(parent);
            }
            let parts: Vec<&str> = rel_path.split("/").collect();
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
                    parent = self.add_node(SiteNode {
                        name: name.to_string(),
                        parent: Some(parent),
                        children: vec![],
                        kind: SiteNodeKind::Folder,
                    })?;
                }
            }
        }
        return Ok(parent);
    }

    pub fn remove(&mut self, id: usize) {
        self.rel_graph.remove_all(id);
        todo!("remove from tree");
    }

    /// Concat resources and minify what can be minified
    pub fn minify(&mut self) {
        todo!()
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
        let mut prev_col = 0;
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
            // if going back fill all the way
            if prev_col > col {
                for _ in amount_rows_in_col..row_length {
                    table[col].push(None);
                }
            } else {
                // if going forward fill to current row - 1
                for _ in amount_rows_in_col + 1..row_length {
                    table[col].push(None);
                }
            }
            prev_col = col;

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
