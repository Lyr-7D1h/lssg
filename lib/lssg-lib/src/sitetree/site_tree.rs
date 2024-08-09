use core::fmt;
use std::{
    collections::HashMap,
    ops::{Index, IndexMut},
};

use log::{debug, warn};

use crate::{tree::Tree, LssgError};

use super::{
    page::Page,
    relational_graph::RelationalGraph,
    relational_graph::{Link, Relation},
    stylesheet::{Stylesheet, StylesheetLink},
    Input, Resource, SiteNode, SiteNodeKind,
};

fn absolute_path(nodes: &Vec<SiteNode>, to: SiteId) -> String {
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
fn rel_path(nodes: &Vec<SiteNode>, from: SiteId, to: SiteId) -> String {
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

pub type SiteId = usize;

/// Code representation of all nodes within the site (hierarchy and how nodes are related)
#[derive(Debug)]
pub struct SiteTree {
    nodes: Vec<SiteNode>,
    root: SiteId,
    // used for detecting if inputs are outside of the root input file
    root_input: Input,

    /// cannonical paths to node ids
    input_to_id: HashMap<Input, SiteId>,
    rel_graph: RelationalGraph,
}

impl SiteTree {
    pub fn len(&self) -> usize {
        return self.nodes.len();
    }

    /// `input` is a markdown input file from where to start discovering resources and pages
    pub fn from_input(input: Input) -> Result<SiteTree, LssgError> {
        let mut tree = SiteTree {
            nodes: vec![],
            root: 0,
            root_input: input.clone(),
            input_to_id: HashMap::new(),
            rel_graph: RelationalGraph::new(),
        };
        tree.add_page_under_parent(input, None)?;
        Ok(tree)
    }

    /// Check if node `id` has `parent_id` as (grand)parent node
    pub fn is_parent(&self, id: SiteId, parent_id: SiteId) -> bool {
        let mut parent = self.nodes[id].parent;
        while let Some(p) = parent {
            if p == parent_id {
                return true;
            }
            parent = self.nodes[id].parent
        }
        return false;
    }

    /// try and get the input of a node if input exists
    pub fn get_input(&self, id: SiteId) -> Option<&Input> {
        self.input_to_id
            .iter()
            .find_map(|(input, i)| if *i == id { Some(input) } else { None })
    }

    // get a node by name by checking the children of `id`
    pub fn get_by_name(&self, name: &str, id: SiteId) -> Option<&SiteId> {
        self.nodes[id]
            .children
            .iter()
            .find(|n| &self.nodes[**n].name == name)
    }

    pub fn root(&self) -> SiteId {
        self.root
    }

    pub fn get(&self, id: SiteId) -> Result<&SiteNode, LssgError> {
        self.nodes.get(id).ok_or(LssgError::sitetree(&format!(
            "Could not find {id} in SiteTree"
        )))
    }

    /// get next parent of page
    pub fn page_parent(&self, id: SiteId) -> Option<SiteId> {
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
    pub fn parents(&self, id: SiteId) -> Vec<SiteId> {
        let mut parent = self.nodes[id].parent;
        let mut parents = vec![];
        while let Some(p) = parent {
            parents.push(p);
            parent = self.nodes[p].parent;
        }
        parents
    }

    /// Get the absolute path of a node
    pub fn path(&self, id: SiteId) -> String {
        absolute_path(&self.nodes, id)
    }

    /// Get the relative path between two nodes
    pub fn rel_path(&self, from: SiteId, to: SiteId) -> String {
        rel_path(&self.nodes, from, to)
    }

    pub fn ids(&self) -> Vec<SiteId> {
        (0..self.nodes.len() - 1).collect()
    }

    /// add a link between two site nodes
    /// This will help create resources necessary for `from`
    pub fn add_link(&mut self, from: SiteId, to: SiteId) {
        self.rel_graph.add(from, to, Relation::External);
    }

    /// Get all the relations from a single node to other nodes
    pub fn links_from(&self, from: SiteId) -> Vec<&Link> {
        self.rel_graph.links_from(from)
    }

    /// Utility function to add a node, create a id and add to parent children
    pub fn add(&mut self, node: SiteNode) -> SiteId {
        // check for name collisions
        if let Some(parent) = node.parent {
            if let Some(id) = self.get_by_name(&node.name, parent) {
                warn!("{} already exists at {id}", node.name);
                return *id;
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

    /// add from Input, will figure out what node to add from input and will register input not to
    /// be used for other nodes
    pub fn add_from_input(
        &mut self,
        input: Input,
        mut parent_id: SiteId,
    ) -> Result<SiteId, LssgError> {
        // return id if file already exists
        if let Some(id) = self.input_to_id.get(&input) {
            warn!(
                "{} already exists using existing node instead",
                input.filename()?
            );
            return Ok(*id);
        }

        let id = if SiteNodeKind::input_is_stylesheet(&input) {
            self.add_stylesheet_from_input(input.clone(), parent_id)?
        } else if SiteNodeKind::input_is_page(&input) {
            self.add_page_from_input(input.clone(), parent_id)?
        } else {
            parent_id = self.create_folders(&input, parent_id)?;
            let id = self.add(SiteNode {
                name: input.filename()?,
                parent: Some(parent_id),
                children: vec![],
                kind: SiteNodeKind::Resource(Resource::new_fetched(input.clone())?),
            });
            self.input_to_id.insert(input.clone(), id);
            id
        };

        Ok(id)
    }

    /// Add a page node to tree and discover any other new pages
    /// will error if input is not a markdown file
    fn add_page_from_input(&mut self, input: Input, parent: SiteId) -> Result<SiteId, LssgError> {
        self.add_page_under_parent(input, Some(parent))
    }

    /// Add a page node to tree and discover any other new pages with possibility of adding root
    fn add_page_under_parent(
        &mut self,
        input: Input,
        mut parent: Option<SiteId>,
    ) -> Result<SiteId, LssgError> {
        if let Some(id) = self.input_to_id.get(&input) {
            // TODO if this page exists should the location of the page be updated?
            return Ok(*id);
        }

        if let Some(parent) = &mut parent {
            *parent = self.create_folders(&input, *parent)?;
        }

        // create early because of the need of an parent id
        let page = Page::from_input(&input)?;
        let id = self.add(SiteNode {
            name: input.filestem().unwrap_or("root".to_string()),
            parent,
            children: vec![],
            kind: SiteNodeKind::Page(page),
        });

        // register input
        self.input_to_id.insert(input.clone(), id);

        let page = match &self.nodes[id].kind {
            SiteNodeKind::Page(page) => page,
            _ => panic!("has to be page"),
        };

        // add other pages
        let links: Vec<(bool, String)> = page
            .links()
            .into_iter()
            .map(|(text, href, ..)| (text.len() == 0, href.clone()))
            .collect();
        for (is_empty, href) in links {
            // if link has no text add whatever is in it
            if is_empty {
                let input = input.new(&href)?;
                let child_id = self.add_from_input(input, id)?;
                self.rel_graph
                    .add(id, child_id, Relation::Discovered { raw_path: href });
                continue;
            }

            if Page::is_href_to_page(&href) {
                let input = input.new(&href)?;
                let child_id = self.add_page_from_input(input, id)?;
                self.rel_graph
                    .add(id, child_id, Relation::Discovered { raw_path: href });
                continue;
            }
        }

        let page = match &self.nodes[id].kind {
            SiteNodeKind::Page(page) => page,
            _ => panic!("has to be page"),
        };
        let images: Vec<String> = page
            .images()
            .into_iter()
            .map(|(_tokens, src, _title)| src.clone())
            .collect();
        for src in images {
            if Input::is_relative(&src) {
                let input = input.new(&src);
                let child_id = self.add_from_input(input?, parent.unwrap_or(self.root))?;
                self.rel_graph
                    .add(id, child_id, Relation::Discovered { raw_path: src });
            }
        }

        return Ok(id);
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    pub fn add_stylesheet_from_input(
        &mut self,
        input: Input,
        mut parent: SiteId,
    ) -> Result<SiteId, LssgError> {
        parent = self.create_folders(&input, parent)?;

        let stylesheet = Stylesheet::try_from(&input)?;
        let stylesheet_links: Vec<String> = stylesheet
            .links()
            .into_iter()
            .map(|p| match p {
                StylesheetLink::Import(s) | StylesheetLink::Url(s) => s.to_string(),
            })
            .collect();

        let parent = self.create_folders(&input, parent)?;
        let stylesheet_id = self.add(SiteNode {
            name: input.filename()?,
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet(stylesheet),
        });

        for link in stylesheet_links {
            let input = input.new(&link)?;
            let parent = self.create_folders(&input, parent)?;
            let resource_id = self.add(SiteNode {
                name: input.filename()?,
                parent: Some(parent),
                children: vec![],
                kind: SiteNodeKind::Resource(Resource::new_fetched(input.clone())?),
            });
            self.rel_graph.add(
                stylesheet_id,
                resource_id,
                Relation::Discovered { raw_path: link },
            );
            self.input_to_id.insert(input, resource_id);
        }

        // register input
        self.input_to_id.insert(input, stylesheet_id);

        Ok(stylesheet_id)
    }

    /// If local input and not outside of `root_input` it will create some extra folders for
    /// structuring SiteTree
    fn create_folders(&mut self, input: &Input, mut parent: SiteId) -> Result<SiteId, LssgError> {
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
                    parent = self.add(SiteNode {
                        name: name.to_string(),
                        parent: Some(parent),
                        children: vec![],
                        kind: SiteNodeKind::Folder,
                    });
                }
            }
        }
        return Ok(parent);
    }

    pub fn remove(&mut self, id: SiteId) {
        self.rel_graph.remove_all(id);
        todo!("remove from tree");
    }

    /// Concat resources and minify what can be minified
    pub fn minify(&mut self) {
        // TODO
        todo!()
    }
}

impl Tree for SiteTree {
    type Node = SiteNode;

    fn root(&self) -> SiteId {
        self.root
    }

    fn get(&self, id: SiteId) -> &Self::Node {
        &self[id]
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

impl Index<SiteId> for SiteTree {
    type Output = SiteNode;

    fn index(&self, index: SiteId) -> &Self::Output {
        &self.nodes[index]
    }
}
impl IndexMut<SiteId> for SiteTree {
    fn index_mut(&mut self, index: SiteId) -> &mut Self::Output {
        &mut self.nodes[index]
    }
}
