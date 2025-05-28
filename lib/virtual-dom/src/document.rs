use std::{collections::HashMap, error::Error};

use crate::Html;

use super::dom_node::DomNode;

#[derive(Debug, Clone)]
/// RefCell based dom tree, tries to mimick Document as seen in browsers (https://developer.mozilla.org/en-US/docs/Web/API/Document)
///
/// using a RC Tree allows for easier manipulation of single nodes and traversing the tree
pub struct Document {
    root: DomNode,
    pub head: DomNode,
    pub body: DomNode,
}

impl Document {
    pub fn from_html(html: Vec<Html>) -> Result<Self, Box<dyn Error>> {
        let root = html.into_iter().nth(1).ok_or("root not found")?;
        let root: DomNode = DomNode::from_html(root).ok_or("invalid root html")?;
        let mut children = root.children();
        let head = children.next().ok_or("head not found")?;
        let body = children.next().ok_or("body not found")?;

        return Ok(Document { root, head, body });
    }

    pub fn new() -> Document {
        let root = DomNode::create_element("html");
        let head = DomNode::create_element("head");
        let body = DomNode::create_element("body");

        root.append_child(head.clone());
        root.append_child(body.clone());

        Document { root, head, body }
    }

    pub fn root(&self) -> DomNode {
        self.root.clone()
    }

    pub fn sanitize(&mut self) {
        self.root.sanitize_children()
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<DomNode> {
        self.root.get_elements_by_tag_name(tag)
    }

    pub fn get_element_by_id(&self, id: &str) -> Option<DomNode> {
        self.root
            .descendants()
            .find(|e| e.get_attribute("id").map(|a| a == id).unwrap_or(false))
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

impl ToString for Document {
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
