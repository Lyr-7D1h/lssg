pub trait Tree {
    type Node: Node;
    fn root(&self) -> usize;
    fn nodes(&self) -> &Vec<Self::Node>;
}

pub trait Node {
    fn children(&self) -> &Vec<usize>;
}

pub struct BFS<'n, T: Tree> {
    stack: Vec<usize>,
    tree: &'n T,
}

impl<'n, T: Tree> BFS<'n, T> {
    pub fn new(tree: &'n T) -> Self {
        let mut stack = Vec::new();
        stack.push(tree.root());
        BFS { stack, tree }
    }
}

impl<'n, T: Tree> Iterator for BFS<'n, T> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.stack.pop() {
            let nodes = self.tree.nodes();
            for child in nodes[id].children() {
                self.stack.push(*child)
            }
            return Some(id);
        }
        None
    }
}