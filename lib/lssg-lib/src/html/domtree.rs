use std::{
    collections::HashMap,
    fmt,
    ops::{Index, IndexMut},
};

use crate::tree::{Node, Tree, DFS};

use super::Html;

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
    parent: Option<usize>,
}

impl Node for DomNode {
    fn children(&self) -> &Vec<usize> {
        &self.children
    }
}

pub type DomId = usize;

/// Tree representation of html (DOM).
///
/// **Will panic if invalid or removed id's are used.**
#[derive(Debug, Clone)]
pub struct DomTree {
    root: DomId,
    nodes: Vec<Option<DomNode>>,
}

impl Tree for DomTree {
    type Node = DomNode;

    fn root(&self) -> DomId {
        self.root
    }

    fn get(&self, id: DomId) -> &DomNode {
        &self[id]
    }
}

impl DomTree {
    pub fn new() -> DomTree {
        let mut tree = DomTree {
            root: 0,
            nodes: vec![Some(DomNode {
                kind: DomNodeKind::Element {
                    tag: "html".to_string(),
                    attributes: HashMap::new(),
                },
                children: vec![],
                parent: None,
            })],
        };
        tree.add_element(tree.root, "head");
        tree.add_element(tree.root, "body");

        return tree;
    }

    pub fn get_mut(&mut self, id: DomId) -> &mut DomNode {
        self.nodes.get_mut(id).unwrap().as_mut().unwrap()
    }

    /// Get all elements with a certain html tag
    pub fn get_elements_by_tag_name(&self, tag_name: impl Into<String>) -> Vec<DomId> {
        let tag_name = tag_name.into();
        return DFS::new(self)
            .filter(|id| {
                if let DomNodeKind::Element { tag, .. } = &self[*id].kind {
                    if tag == &tag_name {
                        return true;
                    }
                }
                false
            })
            .collect();
    }

    /// Add parsed html to tree
    pub fn add_html(&mut self, parent_id: DomId, html: Html) -> Option<usize> {
        match html {
            Html::Comment { .. } => None,
            Html::Text { text } => Some(self.add_text(parent_id, text)),
            Html::Element {
                tag,
                attributes,
                children,
            } => {
                let element = self.add_element_with_attributes(parent_id, tag, attributes);
                for child in children {
                    self.add_html(element, child);
                }
                Some(element)
            }
        }
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add(&mut self, parent_id: DomId, kind: DomNodeKind) -> usize {
        self.nodes.push(Some(DomNode {
            kind,
            children: vec![],
            parent: Some(parent_id),
        }));
        let id = self.nodes.len() - 1;
        self[parent_id].children.push(id);
        id
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add_element(&mut self, parent_id: DomId, tag: impl Into<String>) -> usize {
        self.add(
            parent_id,
            DomNodeKind::Element {
                tag: tag.into(),
                attributes: HashMap::new(),
            },
        )
    }

    pub fn add_element_with_attributes(
        &mut self,
        parent_id: DomId,
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> DomId {
        self.add(
            parent_id,
            DomNodeKind::Element {
                tag: tag.into(),
                attributes,
            },
        )
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add_text(&mut self, parent_id: DomId, text: impl Into<String>) -> usize {
        self.add(parent_id, DomNodeKind::Text { text: text.into() })
    }

    pub fn remove(&mut self, id: DomId) {
        let p = self[id].parent.expect("can't remove root");
        let parent = &mut self[p];
        // remove node from parent
        if let Some(pos) = parent.children.iter().position(|c| *c == id) {
            parent.children.remove(pos);
        }
        // add children to node parent
        let children = self[id].children.clone();
        for c in children.into_iter() {
            (&mut self[p]).children.push(c);
            self[c].parent = Some(p);
        }
        self.nodes[id] = None;
    }

    /// Remove empty tags or invalid html in a way that makes sense
    pub fn validate(&mut self) {
        fn validate_recurs(tree: &mut DomTree, id: DomId) {
            for child in tree[id].children.clone().into_iter() {
                validate_recurs(tree, child);
            }

            let node = &tree[id];

            match &node.kind {
                DomNodeKind::Text { text } => {
                    if text.len() == 0 {
                        tree.remove(id);
                    }
                }
                DomNodeKind::Element { tag, .. } => match tag.as_str() {
                    "p" => {
                        if node.children().len() == 0 {
                            tree.remove(id);
                        }
                    }
                    _ => {}
                },
            }
        }
        validate_recurs(self, self.root());
    }

    fn to_html_content_recurs(&self, index: DomId) -> String {
        let node = &self[index];
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
            let node = &self[n];
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

impl Index<DomId> for DomTree {
    type Output = DomNode;

    fn index(&self, index: DomId) -> &Self::Output {
        self.nodes.get(index).unwrap().as_ref().unwrap()
    }
}
impl IndexMut<DomId> for DomTree {
    fn index_mut(&mut self, index: DomId) -> &mut Self::Output {
        self.nodes.get_mut(index).unwrap().as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove() {
        let mut tree = DomTree::new();
        let body = tree.get_elements_by_tag_name("body")[0];
        let p = tree.add_element(body, "p");
        let text = tree.add_text(p, "This is a paragraph");

        tree.remove(p);
        assert!(tree.nodes.get(p).unwrap().is_none());
        assert_eq!(tree[body].children, vec![text]);
        assert_eq!(tree[text].parent.unwrap(), body);
    }
}
