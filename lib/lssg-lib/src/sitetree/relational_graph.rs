use std::{
    collections::HashSet,
    fmt,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub enum Relation {
    /// from parent to child
    Family,
    External,
    Discovered {
        path: String,
    },
}

#[derive(Debug, Clone)]
pub struct Link {
    from: usize,
    to: usize,
    relation: Relation,
}

/// A directional graph that stores relationships between nodes
#[derive(Debug)]
pub struct RelationalGraph {
    links: Vec<Option<Vec<Link>>>,
}
impl RelationalGraph {
    pub fn new() -> Self {
        RelationalGraph { links: vec![] }
    }

    pub fn add(&mut self, from: usize, to: usize, relation: Relation) {
        // increase size if too short
        let max = from.max(to);
        if self.links.len() < max + 1 {
            for _ in self.links.len()..max + 1 {
                self.links.push(None);
            }
        }

        let link = Link { from, to, relation };
        match self.get_mut(from) {
            Some(links) => links.push(link.clone()),
            None => self.links[from] = Some(vec![link.clone()]),
        }
        match self.get_mut(to) {
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
            links.retain(|l| l.from == from && l.to == to);
        }
        if let Some(links) = self.get_mut(to) {
            links.retain(|l| l.from == from && l.to == to);
        }
    }

    /// remove all links to and from `node_id`
    pub fn remove_all(&mut self, node_id: usize) {
        if let Some(links) = self.get(node_id) {
            for Link { from, to, .. } in links.clone() {
                if from != node_id {
                    self[from].retain(|l| l.from == from && l.to == to);
                } else {
                    self[from].retain(|l| l.from == from && l.to == to);
                }
            }
            self.links[node_id] = None;
        }
    }
}

impl Index<usize> for RelationalGraph {
    type Output = Vec<Link>;

    fn index(&self, index: usize) -> &Self::Output {
        self.links[index].as_ref().unwrap()
    }
}
impl IndexMut<usize> for RelationalGraph {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.links[index].as_mut().unwrap()
    }
}
