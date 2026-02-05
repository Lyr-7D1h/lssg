use core::fmt;
use std::{
    collections::{HashMap, VecDeque},
    iter::once,
    ops::{Index, IndexMut},
};

use log::{debug, info, warn};

use crate::{
    LssgError,
    sitetree::{Javascript, SiteId},
    tree::{Dfs, Tree},
};

use super::{
    Input, Resource, SiteNode, SiteNodeKind,
    page::Page,
    relational_graph::RelationalGraph,
    relational_graph::{Link, Relation},
    stylesheet::{Stylesheet, StylesheetLink},
};

fn absolute_path(nodes: &[SiteNode], to: SiteId) -> String {
    let mut names = vec![nodes[*to].name.clone()];
    let mut parent = nodes[*to].parent;
    while let Some(p) = parent {
        names.push(nodes[*p].name.clone());
        parent = nodes[*p].parent;
    }
    names.pop(); // pop root
    names.reverse();
    format!("/{}", names.join("/"))
}

/// Get the relative path between two nodes
fn rel_path(nodes: &[SiteNode], from: SiteId, to: SiteId) -> String {
    let mut visited = HashMap::new();
    let mut to_path = vec![nodes[*to].name.clone()];

    // discover all parents from destination
    let mut depth = 0;
    let mut node = nodes[*to].parent;
    while let Some(i) = node {
        visited.insert(i, depth);
        depth += 1;
        node = nodes[*i].parent;
        // if not root (root doesn't have a parent) add to file directories
        if nodes[*i].parent.is_some() {
            to_path.push(nodes[*i].name.clone())
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
        node = nodes[*i].parent;
    }

    // don't add anything to path traversal if root
    to_path.reverse();
    let to_path = if nodes[*to].parent.is_some() {
        to_path[to_path.len() - 1 - to_depth..to_path.len()].join("/")
    } else {
        depth -= 1;
        "".into()
    };

    // get remaining path
    if depth > 0 {
        format!("{}{}", "../".repeat(depth), to_path)
    } else {
        format!("./{}", to_path)
    }
}

/// Code representation of all nodes within the site (hierarchy and how nodes are related)
#[derive(Debug)]
pub struct SiteTree {
    nodes: Vec<SiteNode>,
    root: SiteId,
    // used for detecting if inputs are outside of the root input file
    root_input: Input,

    rel_graph: RelationalGraph,
}

impl SiteTree {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// `input` is a markdown input file from where to start discovering resources and pages
    pub fn from_input(input: Input) -> Result<SiteTree, LssgError> {
        let mut tree = SiteTree {
            nodes: vec![],
            root: SiteId(0),
            root_input: input.clone(),
            rel_graph: RelationalGraph::new(),
        };
        tree.add_page_and_discover(input, None)?;
        Ok(tree)
    }

    /// Check if node `id` has `parent_id` as (grand)parent node
    pub fn is_parent(&self, id: SiteId, parent_id: SiteId) -> bool {
        let mut parent = self.nodes[*id].parent;
        while let Some(p) = parent {
            if p == parent_id {
                return true;
            }
            parent = self.nodes[*p].parent
        }
        false
    }

    /// try and get the input of a node if input exists
    pub fn get_input(&self, id: SiteId) -> Option<&Input> {
        match &self.get(id)?.kind {
            SiteNodeKind::Stylesheet(stylesheet) => stylesheet.input(),
            SiteNodeKind::Javascript(javascript) => javascript.input(),
            SiteNodeKind::Page(page) => page.input(),
            SiteNodeKind::Resource(resource) => resource.input(),
            SiteNodeKind::Folder => None,
        }
    }

    fn unitialized(&self) -> bool {
        self.len() == 0
    }

    pub fn get_id_from_input(&self, input: &Input) -> Option<SiteId> {
        if self.unitialized() {
            return None;
        }
        for child in Dfs::new(self) {
            if let Some(child_input) = self.get_input(child)
                && child_input == input
            {
                return Some(child);
            }
        }
        None
    }

    // get a node by name by checking the children of `id`
    pub fn get_by_name(&self, name: &str, id: SiteId) -> Option<&SiteId> {
        self.nodes[*id]
            .children
            .iter()
            .find(|n| self.nodes[***n].name == name)
    }

    pub fn root(&self) -> SiteId {
        self.root
    }

    pub fn get(&self, id: SiteId) -> Option<&SiteNode> {
        self.nodes.get(*id)
    }

    pub fn page(&self, id: SiteId) -> Option<&Page> {
        match &self.get(id)?.kind {
            SiteNodeKind::Page(page) => Some(page),
            _ => None,
        }
    }

    /// get next parent of page
    pub fn page_parent(&self, id: SiteId) -> Option<SiteId> {
        let mut parent = self.nodes[*id].parent;
        let mut parents = vec![];
        while let Some(p) = parent {
            if let SiteNodeKind::Page { .. } = self.nodes[*p].kind {
                return Some(p);
            }
            parents.push(p);
            parent = self.nodes[*p].parent;
        }
        None
    }

    /// Get all parents from a node going up from `id` to root
    pub fn parents(&self, id: SiteId) -> impl Iterator<Item = SiteId> + '_ {
        let initial = self.nodes[*id].parent;
        std::iter::successors(initial, |&p| self.nodes[*p].parent)
    }

    /// Get all children from a node using depth-first search
    pub fn children(&self, id: SiteId) -> impl Iterator<Item = SiteId> + '_ {
        Dfs::from_node(self, id).skip(1)
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
        (0..self.nodes.len()).map(SiteId::from).collect()
    }

    pub fn pages(&self) -> impl Iterator<Item = (SiteId, &Page)> {
        self.nodes.iter().enumerate().filter_map(|(i, node)| {
            if let SiteNodeKind::Page(page) = &node.kind {
                Some((SiteId(i), page))
            } else {
                None
            }
        })
    }

    /// Creates a flattened map where non-page nodes are removed and only pages
    /// are tracked as children. Returns a Vec where each index corresponds to a SiteId
    /// and contains a Vec of its page children (direct or inherited through non-page nodes).
    ///
    /// # Returns
    /// A Vec<Vec<SiteId>> where map[node_id] contains all page children of that node
    pub fn flatten_to_pages(&self) -> Vec<Vec<SiteId>> {
        let mut map = vec![Vec::new(); self.nodes.len()];

        let mut queue = vec![(self.root, self.root)];
        while let Some((id, parent)) = queue.pop() {
            let mut children = vec![];
            for child in &self.nodes[*id].children {
                let child = *child;
                let child_parent = match &self.nodes[*child].kind {
                    SiteNodeKind::Page(_) => {
                        children.push(child);
                        child
                    }
                    _ => parent,
                };
                // Pages become the new parent for their descendants, while non-page nodes
                // inherit the parent from their ancestor page (flattening the hierarchy)
                queue.push((child, child_parent));
            }
            map[*parent].append(&mut children);
        }

        map
    }

    /// add an external relation between two site nodes
    /// This will help create resources necessary for `from`
    pub fn add_link(&mut self, from: SiteId, to: SiteId, relation: Relation) {
        // ignore duplicated
        if self
            .rel_graph
            .links_from(from)
            .iter()
            .any(|l| l.to == to && l.relation == relation)
        {
            return;
        }
        self.rel_graph.add(from, to, relation);
    }

    /// Get all the relations from a single node to other nodes
    pub fn links_from(&self, from: SiteId) -> Vec<&Link> {
        self.rel_graph.links_from(from)
    }

    /// Utility function to add a node, create a id and add to parent children
    pub fn add(&mut self, node: SiteNode) -> SiteId {
        // check for name collisions
        if let Some(parent) = node.parent
            && let Some(id) = self.get_by_name(&node.name, parent)
        {
            warn!("{} already exists at {id}", node.name);
            return *id;
        }

        let id = SiteId::from(self.nodes.len());
        if let Some(parent) = node.parent {
            self.nodes[*parent].children.push(id);
        }
        debug!("Adding {:?} to tree", node.name);
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
        if let Some(id) = self.get_id_from_input(&input) {
            info!(
                "{} already exists using existing node instead",
                input.filename()?
            );
            return Ok(id);
        }

        let id = if SiteNodeKind::input_is_stylesheet(&input) {
            self.add_stylesheet_from_input(input.clone(), parent_id)?
        } else if SiteNodeKind::input_is_page(&input) {
            self.add_page_and_discover(input.clone(), Some(parent_id))?
        } else if SiteNodeKind::input_is_javascript(&input) {
            self.add_javascript_from_input(input, parent_id)?
        } else {
            parent_id = self.create_folders(&input, parent_id);
            let id = self.add(SiteNode {
                name: input.filename()?,
                parent: Some(parent_id),
                children: vec![],
                kind: SiteNodeKind::Resource(Resource::new_fetched(input.clone())),
            });
            id
        };

        Ok(id)
    }

    /// Add a page node to tree and discover any other new pages with possibility of adding root (when parent is `None`)
    fn add_page_and_discover(
        &mut self,
        input: Input,
        parent: Option<SiteId>,
    ) -> Result<SiteId, LssgError> {
        if let Some(id) = self.get_id_from_input(&input) {
            return Ok(id);
        }

        // BFS queue (parent_id, parent_input, href)
        // parent_id is None when root
        // href is None when first page added
        type LinkItem = (Option<SiteId>, Input, Option<String>);

        let mut queue = VecDeque::<LinkItem>::new();
        queue.push_front((parent, input.clone(), None));

        fn visit(
            tree: &mut SiteTree,
            queue: &mut VecDeque<LinkItem>,
        ) -> Result<Option<SiteId>, LssgError> {
            let Some((parent_id, parent_input, href)) = queue.pop_front() else {
                return Ok(None);
            };
            let input = match href.as_ref() {
                Some(href) => match parent_input.join(href) {
                    Ok(input) => input,
                    Err(e) => {
                        warn!("Invalid path {href}: {e}");
                        return Ok(None);
                    }
                },
                None => parent_input,
            };

            let child_id = if SiteNodeKind::input_is_page(&input) {
                if let Some(existing_id) = tree.get_id_from_input(&input) {
                    existing_id
                } else {
                    let new_id = tree.add_page(input.clone(), parent_id)?;
                    // Queue the new page's links and images
                    if let SiteNodeKind::Page(page) = &tree.nodes[*new_id].kind {
                        queue.extend(
                            page.links()
                                .into_iter()
                                .filter_map(|(_, href, ..)| {
                                    if !Input::is_relative(href) {
                                        return None;
                                    }
                                    if !input.join(href).ok().is_some_and(|i| i.exists()) {
                                        log::info!("Ignoring {href}, does not exist or not valid");
                                        return None;
                                    }
                                    Some((Some(new_id), input.clone(), Some(href.clone())))
                                })
                                .chain(page.images().into_iter().map(|(_, src, _)| {
                                    (Some(new_id), input.clone(), Some(src.clone()))
                                })),
                        );
                    }
                    new_id
                }
            } else {
                let Some(parent_id) = parent_id else {
                    log::error!("Cant add resources without a parent: {input:?}");
                    return Ok(None);
                };
                match tree.add_from_input(input.clone(), parent_id) {
                    Ok(id) => id,
                    Err(e) => {
                        log::error!("Failed to add {input}: {e}");
                        return Ok(None);
                    }
                }
            };

            if let Some(parent) = parent_id
                && let Some(href) = href
            {
                tree.rel_graph
                    .add(parent, child_id, Relation::Discovered { raw_path: href });
            }

            visit(tree, queue)?;

            Ok(Some(child_id))
        }

        let id =
            visit(self, &mut queue)?.ok_or(LssgError::io(format!("Failed to add {input:?}")))?;
        Ok(id)
    }

    /// Add a page without discovering others
    fn add_page(&mut self, input: Input, parent: Option<SiteId>) -> Result<SiteId, LssgError> {
        let parent = parent.map(|p| self.create_folders(&input, p));
        let name = input.filestem().unwrap_or("root".to_string());
        let page = Page::try_from(input)?;
        let id = self.add(SiteNode {
            name,
            parent,
            children: vec![],
            kind: SiteNodeKind::Page(page),
        });

        Ok(id)
    }

    fn add_javascript_from_input(
        &mut self,
        input: Input,
        mut parent: SiteId,
    ) -> Result<SiteId, LssgError> {
        parent = self.create_folders(&input, parent);

        let javascript = Javascript::try_from(&input)?;
        let links: Vec<String> = javascript
            .links()
            .into_iter()
            .map(|p| p.to_string())
            .collect();

        let stylesheet_id = self.add(SiteNode {
            name: input.filename()?,
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Javascript(javascript),
        });

        for link in links {
            let input = input.join(&link)?;
            let parent = self.create_folders(&input, parent);
            let resource_id = self.add(SiteNode {
                name: input.filename()?,
                parent: Some(parent),
                children: vec![],
                kind: SiteNodeKind::Resource(Resource::new_fetched(input.clone())),
            });
            self.rel_graph.add(
                stylesheet_id,
                resource_id,
                Relation::Discovered { raw_path: link },
            );
        }

        Ok(stylesheet_id)
    }

    /// Add a stylesheet and all resources needed by the stylesheet
    fn add_stylesheet_from_input(
        &mut self,
        input: Input,
        mut parent: SiteId,
    ) -> Result<SiteId, LssgError> {
        parent = self.create_folders(&input, parent);

        let stylesheet = Stylesheet::try_from(input.clone())?;
        let stylesheet_links: Vec<String> = stylesheet
            .links()
            .into_iter()
            .map(|p| match p {
                StylesheetLink::Import(s) | StylesheetLink::Url(s) => s.to_string(),
            })
            .collect();

        let stylesheet_id = self.add(SiteNode {
            name: input.filename()?,
            parent: Some(parent),
            children: vec![],
            kind: SiteNodeKind::Stylesheet(stylesheet),
        });

        for link in stylesheet_links {
            let input = input.join(&link)?;
            let parent = self.create_folders(&input, parent);
            let resource_id = self.add(SiteNode {
                name: input.filename()?,
                parent: Some(parent),
                children: vec![],
                kind: SiteNodeKind::Resource(Resource::new_fetched(input.clone())),
            });
            self.rel_graph.add(
                stylesheet_id,
                resource_id,
                Relation::Discovered { raw_path: link },
            );
        }

        Ok(stylesheet_id)
    }

    /// Create some extra folders relative to the path and input of the parent
    fn create_folders(&mut self, input: &Input, mut parent: SiteId) -> SiteId {
        let mut base = self.root_input.clone();
        let mut folders = vec![];
        let mut current = parent;
        while let Some(parent) = self[current].parent {
            if parent == self.root {
                break;
            }
            if matches!(self[parent].kind, SiteNodeKind::Folder) {
                folders.push(self[parent].name.as_str());
            }
            if let Some(input) = self.get_input(parent) {
                base = input.clone();
            };
            current = parent;
        }
        folders.reverse();
        base.join(&folders.join("/")).unwrap();

        if let Some(rel_path) = base.make_relative(input) {
            // don't allow backtrack from root
            if rel_path.starts_with("..") {
                return parent;
            }
            let parts: Vec<&str> = rel_path.split("/").collect();
            let parts = &parts[0..parts.len() - 1];

            let mut parents: Vec<SiteId> = once(parent).chain(self.parents(parent)).collect();
            parents.pop();
            parents.reverse();
            for (i, &name) in parts.iter().enumerate() {
                if let Some(parent) = parents.get(i)
                    && self[*parent].name == name
                {
                    continue;
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
        parent
    }

    pub fn remove(&mut self, id: SiteId) {
        self.rel_graph.remove_all(id);
        todo!("soft remove from tree");
    }

    /// Concat resources and minify what can be minified
    pub fn minify(&mut self) {
        // TODO
        todo!()
    }
}

impl Tree<SiteId> for SiteTree {
    type Node = SiteNode;

    fn root(&self) -> SiteId {
        self.root
    }

    fn get(&self, id: SiteId) -> &Self::Node {
        &self.nodes[*id]
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
            let node = &self.nodes[*n];
            for c in &node.children {
                queue.push((*c, col + 1))
            }

            // create col if not exists
            if table.get(col).is_none() {
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

            let node_name = format!("{}({})({})", node.name, n, node.kind);
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
                        if let Some(next_column) = table.get(col + 1)
                            && let Some(Some(_)) = next_column.get(row)
                        {
                            out[row] += " - ";
                            continue;
                        }
                        out[row] += "   ";
                    }
                    None => out[row] += &" ".repeat(max_name_length + 3),
                }
            }
            for item in out.iter_mut().take(row_length).skip(table[col].len()) {
                *item += &" ".repeat(max_name_length + 3);
            }
        }

        f.write_str(&out.join("\n"))?;
        Ok(())
    }
}

impl Index<SiteId> for SiteTree {
    type Output = SiteNode;

    fn index(&self, index: SiteId) -> &Self::Output {
        &self.nodes[*index]
    }
}
impl IndexMut<SiteId> for SiteTree {
    fn index_mut(&mut self, index: SiteId) -> &mut Self::Output {
        &mut self.nodes[*index]
    }
}
