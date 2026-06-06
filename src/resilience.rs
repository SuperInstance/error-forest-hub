//! Mesh resilience analysis: how many failures can the network survive?
//!
//! A mycorrhizal mesh is resilient because it has redundant paths.
//! If one hypha is severed, nutrients (messages) can still flow through
//! alternate routes. This module quantifies that resilience and compares
//! it to a simple tree topology where any single break is catastrophic.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Report on mesh resilience characteristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResilienceReport {
    /// k-resilience: the mesh survives any k link failures without disconnection.
    pub k_resilience: usize,
    /// Links whose removal would disconnect the mesh (bridges).
    pub critical_links: Vec<String>,
    /// Node whose removal most reduces resilience.
    pub bottleneck_node: Option<String>,
    /// Total number of links in the mesh.
    pub total_links: usize,
    /// Total number of nodes in the mesh.
    pub total_nodes: usize,
    /// Whether the mesh is currently connected.
    pub is_connected: bool,
}

/// Represents a link failure event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkFailure {
    pub from: String,
    pub to: String,
    pub link_id: String,
}

impl LinkFailure {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        let from = from.into();
        let to = to.into();
        let link_id = format!("{}-{}", from, to);
        Self { from, to, link_id }
    }
}

/// Resilience analyzer for a mesh.
pub struct ResilienceAnalyzer<'a> {
    mesh: &'a crate::mesh::Mesh,
}

impl<'a> ResilienceAnalyzer<'a> {
    pub fn new(mesh: &'a crate::mesh::Mesh) -> Self {
        Self { mesh }
    }

    /// Generate a full resilience report.
    pub fn analyze(&self) -> ResilienceReport {
        let critical_links = self.find_bridges();
        let bottleneck_node = self.find_bottleneck_node();
        let k_resilience = self.compute_k_resilience();

        ResilienceReport {
            k_resilience,
            critical_links,
            bottleneck_node,
            total_links: self.mesh.links.len(),
            total_nodes: self.mesh.nodes.len(),
            is_connected: self.mesh.is_connected(),
        }
    }

    /// Compute k-resilience: the maximum number of link failures the mesh can
    /// survive while remaining connected. Equal to min-cut size minus 1.
    pub fn compute_k_resilience(&self) -> usize {
        if self.mesh.nodes.len() < 2 {
            return 0;
        }
        if !self.mesh.is_connected() {
            return 0;
        }

        // For each pair of nodes, find the minimum number of links whose
        // removal disconnects them (edge connectivity). The k-resilience
        // is min edge connectivity across all pairs minus 1.
        // Simplified: check if removing any single link disconnects.

        let mut min_connectivity = usize::MAX;
        let node_ids: Vec<String> = self.mesh.nodes.iter().map(|n| n.id.clone()).collect();

        for i in 0..node_ids.len() {
            for j in (i + 1)..node_ids.len() {
                let connectivity = self.edge_connectivity(&node_ids[i], &node_ids[j]);
                min_connectivity = min_connectivity.min(connectivity);
            }
        }

        if min_connectivity == usize::MAX { 0 } else { min_connectivity.saturating_sub(1) }
    }

    /// Find edge connectivity between two nodes (simplified: try removing
    /// increasing numbers of links until they disconnect).
    fn edge_connectivity(&self, a: &str, b: &str) -> usize {
        // Use max-flow via finding all edge-disjoint paths
        let mut count = 0;
        let mut remaining_links: Vec<bool> = vec![true; self.mesh.links.len()];

        loop {
            // Find a path from a to b using only remaining links
            if let Some(path_links) = self.find_path_with_mask(a, b, &remaining_links) {
                count += 1;
                // Remove these links from consideration
                for idx in path_links {
                    remaining_links[idx] = false;
                }
            } else {
                break;
            }
        }
        count
    }

    /// Find a path from `from` to `to` using only links where mask[i] is true.
    /// Returns indices of links used in the path.
    fn find_path_with_mask(&self, from: &str, to: &str, mask: &[bool]) -> Option<Vec<usize>> {
        use std::collections::VecDeque;
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        visited.insert(from.to_string());
        queue.push_back((from.to_string(), Vec::<usize>::new()));

        while let Some((current, path)) = queue.pop_front() {
            if current == to && !path.is_empty() {
                return Some(path);
            }
            for (i, link) in self.mesh.links.iter().enumerate() {
                if !mask[i] { continue; }
                let neighbor = if link.from == current {
                    Some(link.to.clone())
                } else if link.to == current {
                    Some(link.from.clone())
                } else {
                    None
                };
                if let Some(neighbor) = neighbor {
                    if visited.insert(neighbor.clone()) {
                        let mut new_path = path.clone();
                        new_path.push(i);
                        queue.push_back((neighbor, new_path));
                    }
                }
            }
        }
        None
    }

    /// Find bridge links (critical links whose removal disconnects the mesh).
    /// Uses a simple brute-force approach: remove each link and check connectivity.
    pub fn find_bridges(&self) -> Vec<String> {
        let mut bridges = Vec::new();
        if self.mesh.nodes.len() < 2 {
            return bridges;
        }

        for link in &self.mesh.links {
            let modified = self.mesh.without_link_by_endpoints(&link.from, &link.to);
            if !modified.is_connected() {
                bridges.push(link.id());
            }
        }
        bridges
    }

    /// Find the bottleneck node: whose removal most reduces mesh connectivity.
    pub fn find_bottleneck_node(&self) -> Option<String> {
        let mut worst_node = None;
        let mut worst_connectivity = usize::MAX;

        for node in &self.mesh.nodes {
            // Remove this node and all its links, check connectivity
            let mut modified = self.mesh.clone();
            modified.nodes.retain(|n| n.id != node.id);
            modified.links.retain(|l| l.from != node.id && l.to != node.id);

            let components = modified.connected_components();
            let surviving = modified.nodes.len();
            let connectivity_penalty = if surviving == 0 { 0 } else { components };

            if connectivity_penalty < worst_connectivity {
                worst_connectivity = connectivity_penalty;
                worst_node = Some(node.id.clone());
            }
        }

        worst_node
    }

    /// Check if the mesh survives a specific set of link failures.
    pub fn survives_failures(&self, failures: &[LinkFailure]) -> bool {
        let mut modified = self.mesh.clone();
        for failure in failures {
            modified = modified.without_link_by_endpoints(&failure.from, &failure.to);
        }
        modified.is_connected()
    }

    /// Compare resilience with a tree topology of the same nodes.
    /// Returns (mesh_k, tree_k) where tree_k is always 0 (a tree has bridges everywhere).
    pub fn compare_with_tree(&self) -> (usize, usize) {
        let mesh_k = self.compute_k_resilience();
        // A tree on n nodes has exactly n-1 edges, all of which are bridges.
        // So k-resilience of a tree is always 0.
        (mesh_k, 0)
    }
}
