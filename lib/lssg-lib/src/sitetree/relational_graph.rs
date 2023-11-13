pub struct Link {
    from: usize,
    to: usize,
    internal_path: String,
}
pub struct RelationalGraph {
    links: Vec<Option<Vec<Link>>>,
}
impl RelationalGraph {
    pub fn new() -> Self {
        RelationalGraph { links: vec![] }
    }

    pub fn add(&mut self, node_id: usize, link: Link) {
        if node_id > self.links.len() {}
    }

    pub fn get(&self, node_id: usize) -> Option<&Vec<Link>> {
        if let Some(links) = self.links.get(node_id) {
            links.as_ref()
        } else {
            None
        }
    }
    pub fn get_mut(&mut self, node_id: usize) -> Option<&mut Vec<Link>> {
        if let Some(links) = self.links.get_mut(node_id) {
            links.as_mut()
        } else {
            None
        }
    }

    pub fn remove(&mut self, node_id: usize, from: usize, to: usize) {
        if let Some(links) = self.get_mut(node_id) {
            links.retain(|l| l.from == from && l.to == to);
        }
    }
}
