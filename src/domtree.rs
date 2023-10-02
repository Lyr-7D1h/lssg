use std::{collections::HashMap, fmt, str::FromStr};

use crate::{
    lssg_error::LssgError,
    sitetree::SiteNode,
    tree::{Node, Tree, BFS},
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

impl FromStr for DomNode {
    type Err = LssgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
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
        children: Vec<usize>,
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
    nodes: Vec<DomNode>,
}

impl Tree for DomTree {
    type Node = DomNode;

    fn root(&self) -> usize {
        self.root
    }

    fn nodes(&self) -> &Vec<DomNode> {
        &self.nodes
    }
}

impl DomTree {
    pub fn new() -> DomTree {
        let mut tree = DomTree {
            root: 0,
            nodes: vec![DomNode::element_with_attributes("html", HashMap::new())],
        };
        tree.add_element("head", tree.root);
        tree.add_element("body", tree.root);

        return tree;
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut DomNode> {
        return self.nodes.get_mut(id);
    }

    /// Get all elements with a certain html tag
    pub fn get_elements_by_tag_name(&self, tag_name: impl Into<String>) -> Vec<usize> {
        let tag_name = tag_name.into();
        return BFS::new(self)
            .filter(|id| match &self.nodes[*id].kind {
                DomNodeKind::Element { tag, .. } if tag == &tag_name => true,
                _ => false,
            })
            .collect();
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add(&mut self, node: DomNode, parent_id: usize) -> usize {
        self.nodes.push(node);
        let id = self.nodes.len() - 1;
        self.nodes[parent_id].children.push(id);
        id
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

    fn to_html_content_recurs(&self, index: usize) -> String {
        let node = &self.nodes[index];
        match &node.kind {
            DomNodeKind::Text { text } => return text.clone(),
            DomNodeKind::Element { tag, attributes } => {
                let attributes = attributes
                    .into_iter()
                    .map(|(k, v)| {
                        if v.len() > 0 {
                            format!("{k}='{v}'")
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
                    return format!("<{tag}{spacing}{}/>", attributes);
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
        let mut out: String = String::new();

        // TODO use BFS
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
            out += &match &node.kind {
                DomNodeKind::Text { .. } => format!(r#"Text"#),
                DomNodeKind::Element { tag: kind, .. } => kind.to_owned(),
            };
            out += &format!("({})", n);
            current_depth = depth + 1;
        }

        f.write_str(&out)?;
        Ok(())
    }
}

/// Utility function to convert iteratables into attributes hashmap
pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
    arr: I,
) -> HashMap<String, String> {
    arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
}
