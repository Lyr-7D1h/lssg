use std::{collections::HashMap, fmt, str::FromStr};

use crate::{
    lssg_error::LssgError,
    tree::{Node, Tree, DFS},
};

#[derive(Debug, Clone)]
pub enum DomNodeKind {
    Text {
        text: String,
    },
    Element {
        tag: String,
        attributes: HashMap<String, String>,
    },
}

#[derive(Debug, Clone)]
pub struct DomNode {
    pub kind: DomNodeKind,
    children: Vec<usize>,
}

impl DomNode {
    pub fn text(text: impl Into<String>) -> DomNode {
        DomNode {
            kind: DomNodeKind::Text { text: text.into() },
            children: vec![],
        }
    }
    pub fn element(tag: impl Into<String>) -> DomNode {
        DomNode {
            kind: DomNodeKind::Element {
                tag: tag.into(),
                attributes: HashMap::new(),
            },
            children: vec![],
        }
    }
    pub fn element_with_attributes(
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> DomNode {
        DomNode {
            kind: DomNodeKind::Element {
                tag: tag.into(),
                attributes,
            },
            children: vec![],
        }
    }
    pub fn element_with_children(
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
        _children: Vec<usize>,
    ) -> DomNode {
        DomNode {
            kind: DomNodeKind::Element {
                tag: tag.into(),
                attributes,
            },
            children: vec![],
        }
    }
}

impl Node for DomNode {
    fn children(&self) -> &Vec<usize> {
        &self.children
    }
}

#[derive(Debug, Clone)]
pub struct DomTree {
    root: usize,
    nodes: Vec<Option<DomNode>>,
}

impl Tree for DomTree {
    type Node = DomNode;

    fn root(&self) -> usize {
        self.root
    }

    fn get(&self, id: usize) -> Option<&DomNode> {
        match self.nodes.get(id) {
            Some(n) => n.as_ref(),
            None => None,
        }
    }
}

impl DomTree {
    pub fn new() -> DomTree {
        let mut tree = DomTree {
            root: 0,
            nodes: vec![Some(DomNode::element_with_attributes(
                "html",
                HashMap::new(),
            ))],
        };
        tree.add_element("head", tree.root);
        tree.add_element("body", tree.root);

        return tree;
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut DomNode> {
        match self.nodes.get_mut(id) {
            Some(n) => n.as_mut(),
            None => None,
        }
    }

    /// Get all elements with a certain html tag
    pub fn get_elements_by_tag_name(&self, tag_name: impl Into<String>) -> Vec<usize> {
        let tag_name = tag_name.into();
        return DFS::new(self)
            .filter(|id| {
                if let Some(node) = &self.nodes[*id] {
                    if let DomNodeKind::Element { tag, .. } = node.kind {
                        if tag == tag_name {
                            return true;
                        }
                    }
                }
                false
            })
            .collect();
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add(&mut self, node: DomNode, parent_id: usize) -> usize {
        if let Some(parent) = self.nodes[parent_id] {
            self.nodes.push(Some(node));
            let id = self.nodes.len() - 1;
            self.nodes[parent_id].children.push(id);
            Some(id)
        }
        None
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add_element(&mut self, tag: impl Into<String>, parent_id: usize) -> usize {
        self.add(DomNode::element(tag), parent_id)
    }
    pub fn add_element_with_attributes(
        &mut self,
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
        parent_id: usize,
    ) -> usize {
        self.add(DomNode::element_with_attributes(tag, attributes), parent_id)
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add_text(&mut self, text: impl Into<String>, parent_id: usize) -> usize {
        self.add(DomNode::text(text), parent_id)
    }

    pub fn remove(&mut self, id: usize) {
        todo!();
    }

    /// Remove empty tags or invalid html in a way that makes sense
    pub fn validate(&mut self) {
        fn validate_recurs(tree: &mut DomTree, id: usize) {
            for child in tree.nodes[id].children.clone().into_iter() {
                validate_recurs(tree, child);
            }

            let node = &tree.nodes[id];

            match &node.kind {
                DomNodeKind::Text { text } => {
                    if text.len() == 0 {
                        tree.remove(id);
                    }
                }
                DomNodeKind::Element { tag, attributes } => match tag.as_str() {
                    "p" => {
                        if node.children().len() == 0 {
                            tree.remove(id);
                        }
                    }
                    _ => {}
                },
            }
        }
    }

    fn to_html_content_recurs(&self, index: usize) -> String {
        let node = &self.nodes[index];
        match &node.kind {
            DomNodeKind::Text { text } => return text.clone(),
            DomNodeKind::Element { tag, attributes } => {
                let attributes = attributes
                    .into_iter()
                    .map(|(k, v)| {
                        if v.len() > 0 {
                            format!(r#"{k}="{v}""#)
                        } else {
                            k.into()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(" ");

                let spacing = if attributes.len() > 0 {
                    String::from(" ")
                } else {
                    String::new()
                };

                if node.children.len() == 0 {
                    match tag.as_str() {
                        "link" | "meta" => {
                            return format!("<{tag}{spacing}{}/>", attributes);
                        }
                        _ => {}
                    }
                }

                let mut content = String::new();

                for c in &node.children {
                    content += &self.to_html_content_recurs(*c);
                }

                return format!("<{tag}{spacing}{}>{}</{tag}>", attributes, content);
            }
        };
    }

    pub fn to_html_string(self) -> String {
        let html = self.to_html_content_recurs(self.root);
        return format!(r#"<!DOCTYPE html>{html}"#);
    }
}

impl fmt::Display for DomTree {
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

            let name = match &node.kind {
                DomNodeKind::Text { text, .. } => {
                    let mut text = text.clone();
                    text.truncate(10);
                    if text.len() == 10 {
                        format!(r#"{text}.."#)
                    } else {
                        format!(r#"{text}"#)
                    }
                }
                DomNodeKind::Element { tag: kind, .. } => format!("<{}>", kind.to_owned()),
            };
            let node_name = format!("{}({})", name, n);
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

/// Utility function to convert iteratables into attributes hashmap
pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
    arr: I,
) -> HashMap<String, String> {
    arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
}
