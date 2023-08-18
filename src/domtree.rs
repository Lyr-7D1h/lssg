use std::{collections::HashMap, fmt, str::FromStr};

use crate::lssg_error::LssgError;

pub enum DomNodeKind {
    Text {
        text: String,
    },
    Element {
        kind: String,
        attributes: HashMap<String, String>,
    },
}

pub struct DomNode {
    kind: DomNodeKind,
    children: Vec<usize>,
}

impl FromStr for DomNode {
    type Err = LssgError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl DomNode {
    pub fn element(
        kind: impl Into<String>,
        attributes: HashMap<String, String>,
        children: Vec<usize>,
    ) -> DomNode {
        DomNode {
            kind: DomNodeKind::Element {
                kind: kind.into(),
                attributes,
            },
            children,
        }
    }
}

pub struct DomTree {
    root: usize,
    nodes: Vec<DomNode>,
}

impl DomTree {
    pub fn new() -> DomTree {
        let mut tree = DomTree {
            root: 0,
            nodes: vec![DomNode::element("html", HashMap::new(), vec![])],
        };
        tree.add(DomNode::element("head", HashMap::new(), vec![]), tree.root);
        tree.add(DomNode::element("body", HashMap::new(), vec![]), tree.root);

        return tree;
    }

    pub fn filter_by_kind(&self) -> Vec<usize> {
        todo!()
    }

    /// Add a node to the tree return the id (index) of the node
    pub fn add(&mut self, node: DomNode, parent_id: usize) -> usize {
        self.nodes.push(node);
        let id = self.nodes.len() - 1;
        self.nodes[parent_id].children.push(id);
        id
    }

    fn to_html_content_recurs(&self, index: usize) -> String {
        let node = &self.nodes[index];
        return match node.kind {
            DomNodeKind::Text { text } => text,
            DomNodeKind::Element { kind, attributes } => {
                let attributes = attributes
                    .into_iter()
                    .map(|(k, v)| format!("{k}='{v}'"))
                    .collect::<Vec<String>>()
                    .join(" ");

                let spacing = if attributes.len() > 0 {
                    String::from(" ")
                } else {
                    String::new()
                };

                let mut content = String::new();

                for c in &node.children {
                    content += &self.to_html_content_recurs(*c);
                }

                format!("<{kind}{spacing}{}>{}</{kind}>", attributes, content)
            }
        };
    }

    pub fn to_html_content(&self) -> String {
        return self.to_html_content_recurs(self.root);
    }
}

impl fmt::Display for DomTree {
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
            out += match &node.kind {
                DomNodeKind::Text { text } => text,
                DomNodeKind::Element { kind, .. } => kind,
            };
            out += &format!("({})", n);
            current_depth = depth + 1;
        }

        f.write_str(&out)?;
        Ok(())
    }
}
