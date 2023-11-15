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
    pub from: usize,
    pub to: usize,
    pub relation: Relation,
}

/// A directional graph that stores relationships between nodes
#[derive(Debug)]
pub struct RelationalGraph {
    links: Vec<Vec<Link>>,
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
                self.links.push(vec![]);
            }
        }

        let link = Link { from, to, relation };
        self[from].push(link.clone());
        self[to].push(link.clone());
    }

    pub fn links_from(&self, node_id: usize) -> Vec<&Link> {
        self.links[node_id]
            .iter()
            .filter(|l| l.from == node_id)
            .collect()
    }

    pub fn get(&self, node_id: usize) -> &Vec<Link> {
        self.links
            .get(node_id)
            .expect(&format!("{node_id} not found in rel graph"))
    }

    pub fn get_mut(&mut self, node_id: usize) -> &mut Vec<Link> {
        self.links
            .get_mut(node_id)
            .expect(&format!("{node_id} not found in rel graph"))
    }

    pub fn remove(&mut self, from: usize, to: usize) {
        let links = self.get_mut(from);
        links.retain(|l| l.from == from && l.to == to);
        let links = self.get_mut(to);
        links.retain(|l| l.from == from && l.to == to);
    }

    /// remove all links to and from `node_id`
    pub fn remove_all(&mut self, node_id: usize) {
        // remove links pointing to node_id
        for Link { from, to, .. } in self[node_id].clone() {
            if from != node_id {
                self[from].retain(|l| l.from == from && l.to == to);
            }
        }
        self[node_id] = vec![];
    }
}

impl Index<usize> for RelationalGraph {
    type Output = Vec<Link>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.links[index]
    }
}
impl IndexMut<usize> for RelationalGraph {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.links[index]
    }
}
