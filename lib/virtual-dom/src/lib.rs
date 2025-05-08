mod document;
mod dom_node;
mod html;

use std::{collections::VecDeque, iter};

pub use document::*;
pub use dom_node::*;
use html::*;
pub use html::{parse_html, parse_html_from_string, Html};

/// Used in dom-proc for converting braced variables into domnode and to allow any kind of dom element to be appended
#[derive(Debug, PartialEq)]
pub struct IterableNodes(pub Vec<DomNode>);

impl From<String> for IterableNodes {
    fn from(value: String) -> Self {
        Self(vec![DomNode::create_text(value)])
    }
}
impl From<&String> for IterableNodes {
    fn from(value: &String) -> Self {
        Self(vec![DomNode::create_text(value)])
    }
}

impl From<&str> for IterableNodes {
    fn from(value: &str) -> Self {
        Self(vec![DomNode::create_text(value)])
    }
}

impl From<DomNode> for IterableNodes {
    fn from(value: DomNode) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<DomNode>> for IterableNodes {
    fn from(value: Vec<DomNode>) -> Self {
        Self(value)
    }
}

impl FromIterator<DomNode> for IterableNodes {
    fn from_iter<T: IntoIterator<Item = DomNode>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl FromIterator<Html> for IterableNodes {
    fn from_iter<T: IntoIterator<Item = Html>>(iter: T) -> Self {
        iter.into_iter()
            .map(|value| match value {
                Html::Comment { .. } => panic!("root html can't be comment"),
                Html::Text { text } => DomNode::create_text(text),
                Html::Element {
                    tag,
                    attributes,
                    children,
                } => {
                    let root = DomNode::create_element_with_attributes(tag, attributes);
                    let mut queue: VecDeque<(Html, DomNode)> = VecDeque::from(
                        children
                            .into_iter()
                            .zip(std::iter::repeat(root.clone()))
                            .collect::<Vec<(Html, DomNode)>>(),
                    );
                    while let Some((c, parent)) = queue.pop_front() {
                        if let Some(p) = match c {
                            Html::Text { text } => Some(DomNode::create_text(text)),
                            Html::Element {
                                tag,
                                attributes,
                                children,
                            } => {
                                let p = DomNode::create_element_with_attributes(tag, attributes);
                                queue
                                    .extend(children.into_iter().zip(std::iter::repeat(p.clone())));
                                Some(p)
                            }
                            _ => None,
                        } {
                            parent.append_child(p)
                        }
                    }
                    root
                }
            })
            .collect()
    }
}

impl From<Vec<Html>> for IterableNodes {
    fn from(value: Vec<Html>) -> Self {
        value.into_iter().collect()
    }
}

impl From<Html> for IterableNodes {
    fn from(value: Html) -> Self {
        iter::once(value).collect()
    }
}
