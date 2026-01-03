use std::ops::{Index, IndexMut};

use crate::sitetree::SiteId;

#[derive(Debug, Clone)]
pub enum Relation {
    /// Parent-child relationship
    Family,
    /// Relation set by module logic
    External,
    /// Found relation by parsing a file
    Discovered { raw_path: String },
}

#[derive(Debug, Clone)]
pub struct Link {
    pub from: SiteId,
    pub to: SiteId,
    pub relation: Relation,
}

/// A directional graph that stores relationships between nodes
#[derive(Debug)]
pub struct RelationalGraph {
    links: Vec<Vec<Link>>,
}
impl RelationalGraph {
    pub fn new() -> Self {
        RelationalGraph {
            links: vec![vec![]],
        }
    }

    pub fn add(&mut self, from: SiteId, to: SiteId, relation: Relation) {
        // increase size if too short
        let max = from.max(*to);
        if self.links.len() < max + 1 {
            for _ in self.links.len()..max + 1 {
                self.links.push(vec![]);
            }
        }

        let link = Link { from, to, relation };
        self[from].push(link.clone());
        self[to].push(link.clone());
    }

    pub fn links_from(&self, node_id: SiteId) -> Vec<&Link> {
        self.links[*node_id]
            .iter()
            .filter(|l| l.from == node_id)
            .collect()
    }

    /// remove all links to and from `node_id`
    pub fn remove_all(&mut self, node_id: SiteId) {
        // remove links pointing to node_id
        for Link { from, to, .. } in self[node_id].clone() {
            if from != node_id {
                self[from].retain(|l| l.from == from && l.to == to);
            }
        }
        self[node_id] = vec![];
    }
}

impl Index<SiteId> for RelationalGraph {
    type Output = Vec<Link>;

    fn index(&self, index: SiteId) -> &Self::Output {
        &self.links[*index]
    }
}

impl IndexMut<SiteId> for RelationalGraph {
    fn index_mut(&mut self, index: SiteId) -> &mut Self::Output {
        &mut self.links[*index]
    }
}
