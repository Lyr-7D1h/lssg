use std::{fmt, collections::HashSet};

#[derive(Debug)]
pub struct Link {
    to: usize,
    // internal_path: String,
}
impl Link {
    pub fn new(to: usize) -> Self {
        Link { to }
    }
}
#[derive(Debug)]
pub struct RelationalGraph {
    links: Vec<Option<Vec<Link>>>,
}
impl RelationalGraph {
    pub fn new() -> Self {
        RelationalGraph { links: vec![] }
    }

    pub fn add(&mut self, from: usize, to: usize) {
        // increase size if too short
        if self.links.len() < from + 1 {
            for _ in self.links.len()..from + 1 {
                self.links.push(None);
            }
        }
        let link = Link { to };
        match self.get_mut(from) {
            Some(links) => links.push(link),
            None => self.links[from] = Some(vec![link]),
        }
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

    pub fn remove(&mut self, from: usize, to: usize) {
        if let Some(links) = self.get_mut(from) {
            links.retain(|l| l.to == to);
        }
    }
}

impl fmt::Display for RelationalGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out: String = String::new();
        let mut visited = HashSet::new();

        todo!();

        f.write_str(&out)
    }
}
