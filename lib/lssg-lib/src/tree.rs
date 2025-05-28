/// Implement this trait to get generic functionality over tree structures
pub trait Node {
    fn children(&self) -> &Vec<usize>;
}

/// Implement this trait to get generic functionality over tree structures
pub trait Tree {
    type Node: Node;
    fn root(&self) -> usize;
    fn get(&self, id: usize) -> &Self::Node;
}

pub struct DFS<'n, T: Tree> {
    stack: Vec<usize>,
    tree: &'n T,
}

impl<'n, T: Tree> DFS<'n, T> {
    pub fn new(tree: &'n T) -> Self {
        let mut stack = Vec::new();
        stack.push(tree.root());
        DFS { stack, tree }
    }
}

impl<'n, T: Tree> Iterator for DFS<'n, T> {
    type Item = usize;

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
    use crate::tree::DFS;

    use super::{Node, Tree};

    struct TestTree {
        nodes: Vec<TestNode>,
    }
    struct TestNode {
        children: Vec<usize>,
    }
    impl Node for TestNode {
        fn children(&self) -> &Vec<usize> {
            &self.children
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
                },
                TestNode { children: vec![6] },
                TestNode { children: vec![4] },
                TestNode { children: vec![5] },
                TestNode { children: vec![] },
                TestNode { children: vec![] },
                TestNode { children: vec![3] },
            ],
        };

        let order: Vec<usize> = DFS::new(&tree).collect();
        assert_eq!(order, vec![0, 1, 6, 3, 5, 2, 4])
    }
}
