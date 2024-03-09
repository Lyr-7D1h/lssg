use std::collections::HashMap;

use super::dom_node::{DomNode, DomNodeKind};

/// RefCell based dom tree
///
/// using a RC Tree allows for easier manipulation of single nodes and traversing the tree
pub struct DomTree {
    root: DomNode,
    head: DomNode,
    body: DomNode,
}

impl DomTree {
    pub fn new() -> DomTree {
        let root = DomNode::create_element("html");
        let head = DomNode::create_element("head");
        let body = DomNode::create_element("body");

        root.append_child(head.clone());
        root.append_child(body.clone());

        DomTree { root, head, body }
    }

    pub fn root(&self) -> DomNode {
        self.root.clone()
    }

    pub fn head(&self) -> DomNode {
        self.head.clone()
    }

    pub fn body(&self) -> DomNode {
        self.body.clone()
    }

    pub fn sanitize(&mut self) {
        self.root.sanitize_children()
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<DomNode> {
        self.root.get_elements_by_tag_name(tag)
    }

    pub fn create_element(&self, tag: impl Into<String>) -> DomNode {
        DomNode::create_element(tag)
    }

    pub fn create_element_with_attributes(
        &self,
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> DomNode {
        DomNode::create_element_with_attributes(tag, attributes)
    }

    pub fn create_text_node(&self, text: impl Into<String>) -> DomNode {
        DomNode::create_text(text)
    }
}

impl ToString for DomTree {
    fn to_string(&self) -> String {
        format!(r#"<!DOCTYPE html>{}"#, self.root.to_string())
    }
}

/// Utility function to convert iteratables into attributes hashmap
pub fn to_attributes<I: IntoIterator<Item = (impl Into<String>, impl Into<String>)>>(
    arr: I,
) -> HashMap<String, String> {
    arr.into_iter().map(|(k, v)| (k.into(), v.into())).collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tree() {
        let mut tree = DomTree::new();
        // tree.body().append()
    }
}
