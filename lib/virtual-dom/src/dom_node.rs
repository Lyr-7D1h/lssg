use std::cell::{Ref, RefCell, RefMut};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::rc::{Rc, Weak};

use super::Html;

/// Strong link
type Link = Rc<RefCell<DomNodeData>>;
type WeakLink = Weak<RefCell<DomNodeData>>;

/// Based on https://docs.rs/rctree/latest/rctree/struct.Node.html
pub struct DomNode(Link);

pub struct WeakDomNode(WeakLink);

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

struct DomNodeData {
    kind: DomNodeKind,
    parent: Option<WeakLink>,
    first_child: Option<Link>,
    last_child: Option<WeakLink>,
    previous_sibling: Option<WeakLink>,
    next_sibling: Option<Link>,
}

/// Cloning a `Node` only increments a reference count. It does not copy the data.
impl Clone for DomNode {
    fn clone(&self) -> Self {
        DomNode(Rc::clone(&self.0))
    }
}

impl PartialEq for DomNode {
    fn eq(&self, other: &DomNode) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl fmt::Debug for DomNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.kind(), f)
    }
}

impl DomNode {
    /// Creates a new node
    pub fn new(kind: DomNodeKind) -> DomNode {
        DomNode(Rc::new(RefCell::new(DomNodeData {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            previous_sibling: None,
            next_sibling: None,
        })))
    }

    pub fn create_element(tag: impl Into<String>) -> DomNode {
        Self::new(DomNodeKind::Element {
            tag: tag.into(),
            attributes: HashMap::new(),
        })
    }

    pub fn create_element_with_attributes(
        tag: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> DomNode {
        Self::new(DomNodeKind::Element {
            tag: tag.into(),
            attributes,
        })
    }

    pub fn create_text(text: impl Into<String>) -> DomNode {
        Self::new(DomNodeKind::Text { text: text.into() })
    }

    pub fn set_attribute(&mut self, key: String, value: String) {
        if let DomNodeKind::Element { attributes, .. } = &mut *self.kind_mut() {
            attributes.insert(key, value);
        }
    }

    /// Returns a weak referece to a node.
    pub fn downgrade(&self) -> WeakDomNode {
        WeakDomNode(Rc::downgrade(&self.0))
    }

    /// Returns a parent node, unless this node is the root of the tree.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn parent(&self) -> Option<DomNode> {
        Some(DomNode(self.0.borrow().parent.as_ref()?.upgrade()?))
    }

    /// Returns a first child of this node, unless it has no child.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn first_child(&self) -> Option<DomNode> {
        Some(DomNode(self.0.borrow().first_child.as_ref()?.clone()))
    }

    /// Returns a last child of this node, unless it has no child.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn last_child(&self) -> Option<DomNode> {
        Some(DomNode(self.0.borrow().last_child.as_ref()?.upgrade()?))
    }

    /// Returns the previous sibling of this node, unless it is a first child.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn previous_sibling(&self) -> Option<DomNode> {
        Some(DomNode(
            self.0.borrow().previous_sibling.as_ref()?.upgrade()?,
        ))
    }

    /// Returns the next sibling of this node, unless it is a last child.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn next_sibling(&self) -> Option<DomNode> {
        Some(DomNode(self.0.borrow().next_sibling.as_ref()?.clone()))
    }

    pub fn kind(&self) -> Ref<DomNodeKind> {
        Ref::map(self.0.borrow(), |v| &v.kind)
    }

    pub fn kind_mut(&self) -> RefMut<DomNodeKind> {
        RefMut::map(self.0.borrow_mut(), |v| &mut v.kind)
    }

    /// Returns an iterator of nodes to this node and its ancestors.
    ///
    /// Includes the current node.
    pub fn ancestors(&self) -> Ancestors {
        Ancestors(Some(self.clone()))
    }

    /// Returns an iterator of nodes to this node and the siblings before it.
    ///
    /// Includes the current node.
    pub fn preceding_siblings(&self) -> PrecedingSiblings {
        PrecedingSiblings(Some(self.clone()))
    }

    /// Returns an iterator of nodes to this node and the siblings after it.
    ///
    /// Includes the current node.
    pub fn following_siblings(&self) -> FollowingSiblings {
        FollowingSiblings(Some(self.clone()))
    }

    /// Returns an iterator of nodes to this node's children.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn children(&self) -> Children {
        Children {
            next: self.first_child(),
            next_back: self.last_child(),
        }
    }

    /// Returns `true` if this node has children nodes.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn has_children(&self) -> bool {
        self.first_child().is_some()
    }

    /// Returns an iterator of nodes to this node and its descendants, in tree order.
    ///
    /// Includes the current node.
    pub fn descendants(&self) -> Descendants {
        Descendants(self.traverse())
    }

    /// Returns an iterator of nodes to this node and its descendants, in tree order.
    pub fn traverse(&self) -> Traverse {
        Traverse {
            root: self.clone(),
            next: Some(NodeEdge::Start(self.clone())),
            next_back: Some(NodeEdge::End(self.clone())),
        }
    }

    /// Remove empty tags or invalid html in a way that makes sense
    pub fn sanitize_children(&mut self) {
        for mut c in self.children() {
            match &*c.kind() {
                DomNodeKind::Text { text } => {
                    if text.len() == 0 {
                        self.detach();
                        continue;
                    }
                }
                DomNodeKind::Element { tag, .. } => match tag.as_str() {
                    "p" => {
                        // remove paragraph if no children
                        if let None = self.first_child() {
                            self.detach();
                            continue;
                        }
                    }
                    _ => {}
                },
            }
            c.sanitize_children()
        }
    }

    pub fn get_elements_by_tag_name(&self, tag: &str) -> Vec<DomNode> {
        self.descendants()
            .filter(|d| {
                if let DomNodeKind::Element { tag: t, .. } = &*d.kind() {
                    if t == tag {
                        return true;
                    }
                }
                false
            })
            .collect()
    }

    /// Detaches a node from its parent and siblings. Children are not affected.
    ///
    /// # Panics
    ///
    /// Panics if the node or one of its adjoining nodes is currently borrowed.
    pub fn detach(&self) {
        self.0.borrow_mut().detach();
    }

    /// Appends a new child to this node, after existing children.
    ///
    /// # Panics
    ///
    /// Panics if the node, the new child, or one of their adjoining nodes is currently borrowed.
    pub fn append_child(&self, new_child: impl Into<DomNode>) {
        let new_child = new_child.into();
        assert!(*self != new_child, "a node cannot be appended to itself");

        let mut self_borrow = self.0.borrow_mut();
        let mut last_child_opt = None;
        {
            let mut new_child_borrow = new_child.0.borrow_mut();
            new_child_borrow.detach();
            new_child_borrow.parent = Some(Rc::downgrade(&self.0));
            if let Some(last_child_weak) = self_borrow.last_child.take() {
                if let Some(last_child_strong) = last_child_weak.upgrade() {
                    new_child_borrow.previous_sibling = Some(last_child_weak);
                    last_child_opt = Some(last_child_strong);
                }
            }
            self_borrow.last_child = Some(Rc::downgrade(&new_child.0));
        }

        if let Some(last_child_strong) = last_child_opt {
            let mut last_child_borrow = last_child_strong.borrow_mut();
            debug_assert!(last_child_borrow.next_sibling.is_none());
            last_child_borrow.next_sibling = Some(new_child.0);
        } else {
            // No last child
            debug_assert!(self_borrow.first_child.is_none());
            self_borrow.first_child = Some(new_child.0);
        }
    }

    /// Prepends a new child to this node, before existing children.
    ///
    /// # Panics
    ///
    /// Panics if the node, the new child, or one of their adjoining nodes is currently borrowed.
    pub fn prepend(&self, new_child: DomNode) {
        assert!(*self != new_child, "a node cannot be prepended to itself");

        let mut self_borrow = self.0.borrow_mut();
        {
            let mut new_child_borrow = new_child.0.borrow_mut();
            new_child_borrow.detach();
            new_child_borrow.parent = Some(Rc::downgrade(&self.0));
            match self_borrow.first_child.take() {
                Some(first_child_strong) => {
                    {
                        let mut first_child_borrow = first_child_strong.borrow_mut();
                        debug_assert!(first_child_borrow.previous_sibling.is_none());
                        first_child_borrow.previous_sibling = Some(Rc::downgrade(&new_child.0));
                    }
                    new_child_borrow.next_sibling = Some(first_child_strong);
                }
                None => {
                    debug_assert!(self_borrow.first_child.is_none());
                    self_borrow.last_child = Some(Rc::downgrade(&new_child.0));
                }
            }
        }
        self_borrow.first_child = Some(new_child.0);
    }

    /// Inserts a new sibling after this node.
    ///
    /// # Panics
    ///
    /// Panics if the node, the new sibling, or one of their adjoining nodes is currently borrowed.
    pub fn insert_after(&self, new_sibling: DomNode) {
        assert!(
            *self != new_sibling,
            "a node cannot be inserted after itself"
        );

        let mut self_borrow = self.0.borrow_mut();
        {
            let mut new_sibling_borrow = new_sibling.0.borrow_mut();
            new_sibling_borrow.detach();
            new_sibling_borrow.parent = self_borrow.parent.clone();
            new_sibling_borrow.previous_sibling = Some(Rc::downgrade(&self.0));
            match self_borrow.next_sibling.take() {
                Some(next_sibling_strong) => {
                    {
                        let mut next_sibling_borrow = next_sibling_strong.borrow_mut();
                        debug_assert!({
                            let weak = next_sibling_borrow.previous_sibling.as_ref().unwrap();
                            Rc::ptr_eq(&weak.upgrade().unwrap(), &self.0)
                        });
                        next_sibling_borrow.previous_sibling = Some(Rc::downgrade(&new_sibling.0));
                    }
                    new_sibling_borrow.next_sibling = Some(next_sibling_strong);
                }
                None => {
                    if let Some(parent_ref) = self_borrow.parent.as_ref() {
                        if let Some(parent_strong) = parent_ref.upgrade() {
                            let mut parent_borrow = parent_strong.borrow_mut();
                            parent_borrow.last_child = Some(Rc::downgrade(&new_sibling.0));
                        }
                    }
                }
            }
        }
        self_borrow.next_sibling = Some(new_sibling.0);
    }

    /// Inserts a new sibling before this node.
    ///
    /// # Panics
    ///
    /// Panics if the node, the new sibling, or one of their adjoining nodes is currently borrowed.
    pub fn insert_before(&self, new_sibling: DomNode) {
        assert!(
            *self != new_sibling,
            "a node cannot be inserted before itself"
        );

        let mut self_borrow = self.0.borrow_mut();
        let mut previous_sibling_opt = None;
        {
            let mut new_sibling_borrow = new_sibling.0.borrow_mut();
            new_sibling_borrow.detach();
            new_sibling_borrow.parent = self_borrow.parent.clone();
            new_sibling_borrow.next_sibling = Some(self.0.clone());
            if let Some(previous_sibling_weak) = self_borrow.previous_sibling.take() {
                if let Some(previous_sibling_strong) = previous_sibling_weak.upgrade() {
                    new_sibling_borrow.previous_sibling = Some(previous_sibling_weak);
                    previous_sibling_opt = Some(previous_sibling_strong);
                }
            }
            self_borrow.previous_sibling = Some(Rc::downgrade(&new_sibling.0));
        }

        if let Some(previous_sibling_strong) = previous_sibling_opt {
            let mut previous_sibling_borrow = previous_sibling_strong.borrow_mut();
            debug_assert!({
                let rc = previous_sibling_borrow.next_sibling.as_ref().unwrap();
                Rc::ptr_eq(rc, &self.0)
            });
            previous_sibling_borrow.next_sibling = Some(new_sibling.0);
        } else {
            // No previous sibling.
            if let Some(parent_ref) = self_borrow.parent.as_ref() {
                if let Some(parent_strong) = parent_ref.upgrade() {
                    let mut parent_borrow = parent_strong.borrow_mut();
                    parent_borrow.first_child = Some(new_sibling.0);
                }
            }
        }
    }
}

impl ToString for DomNode {
    fn to_string(&self) -> String {
        match &*self.kind() {
            DomNodeKind::Text { text } => text.to_string(),
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

                let children: Vec<DomNode> = self.children().collect();
                if children.len() == 0 {
                    if is_void_element(&tag) {
                        return format!("<{tag}{spacing}{}/>", attributes);
                    }
                }

                let mut content = String::new();

                for c in children {
                    content += &c.to_string();
                }

                format!("<{tag}{spacing}{}>{}</{tag}>", attributes, content)
            }
        }
    }
}

impl From<Html> for DomNode {
    fn from(value: Html) -> Self {
        match value {
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
                            queue.extend(children.into_iter().zip(std::iter::repeat(p.clone())));
                            Some(p)
                        }
                        _ => None,
                    } {
                        parent.append_child(p)
                    }
                }
                root
            }
        }
    }
}

/// check if a html tag is a void tag (it can not have children)
pub fn is_void_element(tag: &str) -> bool {
    match tag {
        "base" | "img" | "br" | "col" | "embed" | "hr" | "area" | "input" | "link" | "meta"
        | "param" | "source" | "track" | "wbr" => true,
        _ => false,
    }
}

/// Cloning a `WeakNode` only increments a reference count. It does not copy the data.
impl Clone for WeakDomNode {
    fn clone(&self) -> Self {
        WeakDomNode(Weak::clone(&self.0))
    }
}

impl fmt::Debug for WeakDomNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("(WeakNode)")
    }
}

impl WeakDomNode {
    /// Attempts to upgrade the WeakNode to a Node.
    pub fn upgrade(&self) -> Option<DomNode> {
        self.0.upgrade().map(DomNode)
    }
}

impl DomNodeData {
    /// Detaches a node from its parent and siblings. Children are not affected.
    fn detach(&mut self) {
        let parent_weak = self.parent.take();
        let previous_sibling_weak = self.previous_sibling.take();
        let next_sibling_strong = self.next_sibling.take();

        let previous_sibling_opt = previous_sibling_weak
            .as_ref()
            .and_then(|weak| weak.upgrade());

        if let Some(next_sibling_ref) = next_sibling_strong.as_ref() {
            let mut next_sibling_borrow = next_sibling_ref.borrow_mut();
            next_sibling_borrow.previous_sibling = previous_sibling_weak;
        } else if let Some(parent_ref) = parent_weak.as_ref() {
            if let Some(parent_strong) = parent_ref.upgrade() {
                let mut parent_borrow = parent_strong.borrow_mut();
                parent_borrow.last_child = previous_sibling_weak;
            }
        }

        if let Some(previous_sibling_strong) = previous_sibling_opt {
            let mut previous_sibling_borrow = previous_sibling_strong.borrow_mut();
            previous_sibling_borrow.next_sibling = next_sibling_strong;
        } else if let Some(parent_ref) = parent_weak.as_ref() {
            if let Some(parent_strong) = parent_ref.upgrade() {
                let mut parent_borrow = parent_strong.borrow_mut();
                parent_borrow.first_child = next_sibling_strong;
            }
        }
    }
}

impl Drop for DomNodeData {
    fn drop(&mut self) {
        // Collect all descendant nodes and detach them to prevent the stack overflow.

        let mut stack = Vec::new();
        if let Some(first_child) = self.first_child.as_ref() {
            // Create `Node` from `NodeData`.
            let first_child = DomNode(first_child.clone());
            // Iterate `self` children, without creating yet another `Node`.
            for child1 in first_child.following_siblings() {
                for child2 in child1.descendants() {
                    stack.push(child2);
                }
            }
        }

        for node in stack {
            node.detach();
        }
    }
}

// /// Iterators prelude.
// pub mod iterator {
//     pub use super::Ancestors;
//     pub use super::Children;
//     pub use super::Descendants;
//     pub use super::FollowingSiblings;
//     pub use super::NodeEdge;
//     pub use super::PrecedingSiblings;
//     pub use super::Traverse;
// }

macro_rules! impl_node_iterator {
    ($name: ident, $next: expr) => {
        impl Iterator for $name {
            type Item = DomNode;

            /// # Panics
            ///
            /// Panics if the node about to be yielded is currently mutably borrowed.
            fn next(&mut self) -> Option<Self::Item> {
                match self.0.take() {
                    Some(node) => {
                        self.0 = $next(&node);
                        Some(node)
                    }
                    None => None,
                }
            }
        }
    };
}

/// An iterator of nodes to the ancestors a given node.
pub struct Ancestors(Option<DomNode>);
impl_node_iterator!(Ancestors, |node: &DomNode| node.parent());

/// An iterator of nodes to the siblings before a given node.
pub struct PrecedingSiblings(Option<DomNode>);
impl_node_iterator!(PrecedingSiblings, |node: &DomNode| node.previous_sibling());

/// An iterator of nodes to the siblings after a given node.
pub struct FollowingSiblings(Option<DomNode>);
impl_node_iterator!(FollowingSiblings, |node: &DomNode| node.next_sibling());

/// A double ended iterator of nodes to the children of a given node.
pub struct Children {
    next: Option<DomNode>,
    next_back: Option<DomNode>,
}

impl Children {
    // true if self.next_back's next sibling is self.next
    fn finished(&self) -> bool {
        match self.next_back {
            Some(ref next_back) => next_back.next_sibling() == self.next,
            _ => true,
        }
    }
}

impl Iterator for Children {
    type Item = DomNode;

    /// # Panics
    ///
    /// Panics if the node about to be yielded is currently mutably borrowed.
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished() {
            return None;
        }

        match self.next.take() {
            Some(node) => {
                self.next = node.next_sibling();
                Some(node)
            }
            None => None,
        }
    }
}

impl DoubleEndedIterator for Children {
    /// # Panics
    ///
    /// Panics if the node about to be yielded is currently mutably borrowed.
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.finished() {
            return None;
        }

        match self.next_back.take() {
            Some(node) => {
                self.next_back = node.previous_sibling();
                Some(node)
            }
            None => None,
        }
    }
}

/// An iterator of nodes to a given node and its descendants, in tree order.
pub struct Descendants(Traverse);

impl Iterator for Descendants {
    type Item = DomNode;

    /// # Panics
    ///
    /// Panics if the node about to be yielded is currently mutably borrowed.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.0.next() {
                Some(NodeEdge::Start(node)) => return Some(node),
                Some(NodeEdge::End(_)) => {}
                None => return None,
            }
        }
    }
}

/// A node type during traverse.
#[derive(Clone, Debug)]
pub enum NodeEdge {
    /// Indicates that start of a node that has children.
    /// Yielded by `Traverse::next` before the node's descendants.
    /// In HTML or XML, this corresponds to an opening tag like `<div>`
    Start(DomNode),

    /// Indicates that end of a node that has children.
    /// Yielded by `Traverse::next` after the node's descendants.
    /// In HTML or XML, this corresponds to a closing tag like `</div>`
    End(DomNode),
}

// Implement PartialEq manually, because we do not need to require T: PartialEq
impl PartialEq for NodeEdge {
    fn eq(&self, other: &NodeEdge) -> bool {
        match (self, other) {
            (&NodeEdge::Start(ref n1), &NodeEdge::Start(ref n2)) => *n1 == *n2,
            (&NodeEdge::End(ref n1), &NodeEdge::End(ref n2)) => *n1 == *n2,
            _ => false,
        }
    }
}

impl NodeEdge {
    fn next_item(&self, root: &DomNode) -> Option<NodeEdge> {
        match *self {
            NodeEdge::Start(ref node) => match node.first_child() {
                Some(first_child) => Some(NodeEdge::Start(first_child)),
                None => Some(NodeEdge::End(node.clone())),
            },
            NodeEdge::End(ref node) => {
                if *node == *root {
                    None
                } else {
                    match node.next_sibling() {
                        Some(next_sibling) => Some(NodeEdge::Start(next_sibling)),
                        // `node.parent()` here can only be `None`
                        // if the tree has been modified during iteration,
                        // but silently stopping iteration
                        // seems a more sensible behavior than panicking.
                        None => node.parent().map(NodeEdge::End),
                    }
                }
            }
        }
    }

    fn previous_item(&self, root: &DomNode) -> Option<NodeEdge> {
        match *self {
            NodeEdge::End(ref node) => match node.last_child() {
                Some(last_child) => Some(NodeEdge::End(last_child)),
                None => Some(NodeEdge::Start(node.clone())),
            },
            NodeEdge::Start(ref node) => {
                if *node == *root {
                    None
                } else {
                    match node.previous_sibling() {
                        Some(previous_sibling) => Some(NodeEdge::End(previous_sibling)),
                        // `node.parent()` here can only be `None`
                        // if the tree has been modified during iteration,
                        // but silently stopping iteration
                        // seems a more sensible behavior than panicking.
                        None => node.parent().map(NodeEdge::Start),
                    }
                }
            }
        }
    }
}

/// A double ended iterator of nodes to a given node and its descendants,
/// in tree order.
pub struct Traverse {
    root: DomNode,
    next: Option<NodeEdge>,
    next_back: Option<NodeEdge>,
}

impl Traverse {
    // true if self.next_back's next item is self.next
    fn finished(&self) -> bool {
        match self.next_back {
            Some(ref next_back) => next_back.next_item(&self.root) == self.next,
            _ => true,
        }
    }
}

impl Iterator for Traverse {
    type Item = NodeEdge;

    /// # Panics
    ///
    /// Panics if the node about to be yielded is currently mutably borrowed.
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished() {
            return None;
        }

        match self.next.take() {
            Some(item) => {
                self.next = item.next_item(&self.root);
                Some(item)
            }
            None => None,
        }
    }
}

impl DoubleEndedIterator for Traverse {
    /// # Panics
    ///
    /// Panics if the node about to be yielded is currently mutably borrowed.
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.finished() {
            return None;
        }

        match self.next_back.take() {
            Some(item) => {
                self.next_back = item.previous_item(&self.root);
                Some(item)
            }
            None => None,
        }
    }
}
