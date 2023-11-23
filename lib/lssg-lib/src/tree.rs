/// Implement this trait to get generic functionality over tree structures
pub trait Node {
    fn children(&self) -> &Vec<usize>;
}

/// Implement this trait to get generic functionality over tree structures
pub trait Tree {
    type Node: Node;
    fn root(&self) -> usize;
    fn get(&self, id: usize) -> Option<&Self::Node>;
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
            if let Some(node) = self.tree.get(id) {
            for child in node.children() {
                self.stack.push(*child)
            }
                
            }
            return Some(id);
        }
        None
    }
}
