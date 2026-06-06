//! Mycorrhizal mesh topology: nodes connected by fungal hyphae links.
//!
//! In nature, mycorrhizal networks connect trees underground through fungal
//! hyphae, forming a resilient mesh that can reroute nutrients around damage.
//! This module models that topology for error-correcting message relay.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// A node in the mycorrhizal mesh.
///
/// Hub nodes act as major relay points (like mother trees in a fungal network),
/// while leaf nodes are endpoints that originate or consume messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeshNode {
    pub id: String,
    pub is_hub: bool,
    pub position: (f64, f64),
}

impl MeshNode {
    pub fn new(id: impl Into<String>, is_hub: bool, position: (f64, f64)) -> Self {
        Self { id: id.into(), is_hub, position }
    }

    /// Euclidean distance to another node.
    pub fn distance_to(&self, other: &MeshNode) -> f64 {
        let dx = self.position.0 - other.position.0;
        let dy = self.position.1 - other.position.1;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A single mesh containing nodes and their hyphae links.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Mesh {
    pub nodes: Vec<MeshNode>,
    pub links: Vec<crate::hyphae_link::HyphaeLink>,
}

impl Mesh {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), links: Vec::new() }
    }

    pub fn with_nodes(mut self, nodes: Vec<MeshNode>) -> Self {
        self.nodes = nodes;
        self
    }

    pub fn with_links(mut self, links: Vec<crate::hyphae_link::HyphaeLink>) -> Self {
        self.links = links;
        self
    }

    pub fn add_node(&mut self, node: MeshNode) {
        self.nodes.push(node);
    }

    pub fn add_link(&mut self, link: crate::hyphae_link::HyphaeLink) {
        self.links.push(link);
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&MeshNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all hub nodes.
    pub fn hub_nodes(&self) -> Vec<&MeshNode> {
        self.nodes.iter().filter(|n| n.is_hub).collect()
    }

    /// Get neighbors of a node (undirected — links go both ways).
    pub fn neighbors(&self, node_id: &str) -> Vec<&str> {
        self.links
            .iter()
            .filter_map(|l| {
                if l.from == node_id {
                    Some(l.to.as_str())
                } else if l.to == node_id {
                    Some(l.from.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the mesh is fully connected (every node can reach every other node).
    pub fn is_connected(&self) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        let visited = self.bfs_from(&self.nodes[0].id);
        visited.len() == self.nodes.len()
    }

    /// BFS from a given node, returns set of reachable node IDs.
    pub fn bfs_from(&self, start: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start.to_string());
        visited.insert(start.to_string());

        while let Some(current) = queue.pop_front() {
            for neighbor in self.neighbors(&current) {
                let n = neighbor.to_string();
                if visited.insert(n.clone()) {
                    queue.push_back(n);
                }
            }
        }
        visited
    }

    /// Get adjacency map: node_id → list of (neighbor_id, link_index).
    pub fn adjacency(&self) -> HashMap<String, Vec<(String, usize)>> {
        let mut adj: HashMap<String, Vec<(String, usize)>> = HashMap::new();
        for (i, link) in self.links.iter().enumerate() {
            adj.entry(link.from.clone()).or_default().push((link.to.clone(), i));
            adj.entry(link.to.clone()).or_default().push((link.from.clone(), i));
        }
        adj
    }

    /// Remove a link by index and return a new mesh without it.
    pub fn without_link(&self, index: usize) -> Self {
        let mut new_mesh = self.clone();
        if index < new_mesh.links.len() {
            new_mesh.links.remove(index);
        }
        new_mesh
    }

    /// Remove a link by endpoints.
    pub fn without_link_by_endpoints(&self, from: &str, to: &str) -> Self {
        let mut new_mesh = self.clone();
        new_mesh.links.retain(|l| {
            !((l.from == from && l.to == to) || (l.from == to && l.to == from))
        });
        new_mesh
    }

    /// Count connected components.
    pub fn connected_components(&self) -> usize {
        let mut visited = HashSet::new();
        let mut count = 0;
        for node in &self.nodes {
            if !visited.contains(&node.id) {
                count += 1;
                let reachable = self.bfs_from(&node.id);
                visited.extend(reachable);
            }
        }
        count
    }
}

impl Default for Mesh {
    fn default() -> Self {
        Self::new()
    }
}
