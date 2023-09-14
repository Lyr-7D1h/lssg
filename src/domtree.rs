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
    pub fn element(kind: impl Into<String>, attributes: HashMap<String, String>) -> DomNode {
        DomNode {
            kind: DomNodeKind::Element {
                kind: kind.into(),
                attributes,
            },
            children: vec![],
        }
    }
}

pub struct DomTree {
    root: usize,
    head: usize,
    body: usize,
    nodes: Vec<DomNode>,
}

impl DomTree {
    pub fn new(lang: String) -> DomTree {
        let mut html_attributes = HashMap::new();
        html_attributes.insert("lang".to_string(), lang);

        let mut tree = DomTree {
            root: 0,
            head: 1,
            body: 2,
            nodes: vec![DomNode::element("html", html_attributes)],
        };
        tree.add(DomNode::element("head", HashMap::new()), tree.root);
        tree.add(DomNode::element("body", HashMap::new()), tree.root);

        return tree;
    }

    pub fn head(&self) -> usize {
        self.head
    }
    pub fn body(&self) -> usize {
        self.body
    }
    pub fn root(&self) -> usize {
        self.root
    }

    /// Breadth first search
    fn bfs(self, cb: impl Fn(&DomNode) -> bool) -> Option<usize> {
        let mut queue = vec![self.root];
        while let Some(i) = queue.pop() {
            let node = &self.nodes[i];
            for c in node.children.iter() {
                queue.push(*c);
            }
            if cb(node) {
                return Some(i);
            }
        }
        return None;
    }

    pub fn find_element_by_kind(self, target_kind: impl Into<String>) -> Option<usize> {
        let target_kind: String = target_kind.into();
        return self.bfs(|node| match &node.kind {
            DomNodeKind::Element { kind, .. } => {
                if kind == &target_kind {
                    true
                } else {
                    false
                }
            }
            _ => false,
        });
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
        let html = self.to_html_content_recurs(self.root);
        return format!("<!DOCTYPE html>{html}");
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
