/// Implement this trait to get generic functionality over tree structures
pub trait Node<Id = usize> {
    fn children(&self) -> &Vec<Id>;
    fn parent(&self) -> Option<Id>;
}

/// Implement this trait to get generic functionality over tree structures
pub trait Tree<Id = usize> {
    type Node: Node<Id>;
    /// None if unitialized
    fn root(&self) -> Id;
    fn get(&self, id: Id) -> &Self::Node;
}

pub struct Dfs<'n, Id, T: Tree<Id>> {
    stack: Vec<Id>,
    tree: &'n T,
}

impl<'n, Id, T: Tree<Id>> Dfs<'n, Id, T> {
    pub fn new(tree: &'n T) -> Self {
        let stack = vec![tree.root()];
        Dfs { stack, tree }
    }

    pub fn from_node(tree: &'n T, start: Id) -> Self {
        let stack = vec![start];
        Dfs { stack, tree }
    }
}

impl<'n, Id: Copy, T: Tree<Id>> Iterator for Dfs<'n, Id, T> {
    type Item = Id;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.stack.pop() {
            let node = self.tree.get(id);
            for child in node.children() {
                self.stack.push(*child)
            }
            return Some(id);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::tree::Dfs;

    use super::{Node, Tree};

    struct TestTree {
        nodes: Vec<TestNode>,
    }
    struct TestNode {
        children: Vec<usize>,
        parent: Option<usize>,
    }
    impl Node for TestNode {
        fn children(&self) -> &Vec<usize> {
            &self.children
        }

        fn parent(&self) -> Option<usize> {
            self.parent
        }
    }
    impl Tree for TestTree {
        type Node = TestNode;

        fn root(&self) -> usize {
            return 0;
        }

        fn get(&self, id: usize) -> &Self::Node {
            &self.nodes[id]
        }
    }

    #[test]
    fn in_parent_order() {
        let tree = TestTree {
            nodes: vec![
                TestNode {
                    children: vec![2, 1],
                    parent: None,
                },
                TestNode {
                    children: vec![6],
                    parent: Some(0),
                },
                TestNode {
                    children: vec![4],
                    parent: Some(0),
                },
                TestNode {
                    children: vec![5],
                    parent: Some(6),
                },
                TestNode {
                    children: vec![],
                    parent: Some(2),
                },
                TestNode {
                    children: vec![],
                    parent: Some(3),
                },
                TestNode {
                    children: vec![3],
                    parent: Some(1),
                },
            ],
        };

        let order: Vec<usize> = Dfs::new(&tree).collect();
        assert_eq!(order, vec![0, 1, 6, 3, 5, 2, 4])
    }
}
