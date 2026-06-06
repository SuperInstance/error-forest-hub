//! Mesh routing: find paths through the mycorrhizal network.
//!
//! Like nutrients finding their way through a fungal network, messages
//! must be routed optimally. We offer three strategies inspired by
//! how real mycorrhizal networks adapt to conditions.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

/// A computed route through the mesh.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Route {
    pub path: Vec<String>,
    pub total_latency: f64,
    pub reliability: f64,
    pub hops: usize,
}

impl Route {
    pub fn new(path: Vec<String>, total_latency: f64, reliability: f64) -> Self {
        let hops = if path.is_empty() { 0 } else { path.len() - 1 };
        Self { path, total_latency, reliability, hops }
    }

    /// An empty route representing no path found.
    pub fn none() -> Self {
        Self { path: vec![], total_latency: f64::INFINITY, reliability: 0.0, hops: 0 }
    }

    pub fn is_valid(&self) -> bool {
        !self.path.is_empty()
    }
}

/// Routing algorithm variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RoutingAlgorithm {
    /// Shortest path by latency (standard Dijkstra).
    ShortestPath,
    /// Most-reliable path (maximize product of link reliabilities).
    MostReliable,
    /// Widest path: maximize minimum bandwidth along the path (maximin).
    WidestPath,
}

/// Internal struct for Dijkstra's priority queue.
#[derive(Debug, Clone)]
struct DijkstraEntry {
    node: String,
    cost: f64,
    path: Vec<String>,
}

impl PartialEq for DijkstraEntry {
    fn eq(&self, other: &Self) -> bool { self.cost == other.cost }
}
impl Eq for DijkstraEntry {}
impl PartialOrd for DijkstraEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for DijkstraEntry {
    fn cmp(&self, other: &Self) -> Ordering { other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal) }
}

/// Router that finds paths through a mesh.
pub struct MeshRouter<'a> {
    mesh: &'a crate::mesh::Mesh,
}

impl<'a> MeshRouter<'a> {
    pub fn new(mesh: &'a crate::mesh::Mesh) -> Self {
        Self { mesh }
    }

    /// Find a route using the specified algorithm.
    pub fn route(&self, from: &str, to: &str, algorithm: &RoutingAlgorithm) -> Route {
        match algorithm {
            RoutingAlgorithm::ShortestPath => self.shortest_path(from, to),
            RoutingAlgorithm::MostReliable => self.most_reliable_path(from, to),
            RoutingAlgorithm::WidestPath => self.widest_path(from, to),
        }
    }

    /// Shortest path by latency using Dijkstra.
    pub fn shortest_path(&self, from: &str, to: &str) -> Route {
        let adj = self.mesh.adjacency();
        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut prev: HashMap<String, String> = HashMap::new();
        let mut heap = BinaryHeap::new();

        for node in &self.mesh.nodes {
            dist.insert(node.id.clone(), f64::INFINITY);
        }
        dist.insert(from.to_string(), 0.0);
        heap.push(DijkstraEntry {
            node: from.to_string(),
            cost: 0.0,
            path: vec![from.to_string()],
        });

        while let Some(entry) = heap.pop() {
            if entry.node == to {
                return self.build_route(&entry.path, |link| link.latency, |link| link.reliability());
            }
            if entry.cost > dist.get(&entry.node).copied().unwrap_or(f64::INFINITY) {
                continue;
            }
            if let Some(neighbors) = adj.get(&entry.node) {
                for (neighbor, link_idx) in neighbors {
                    let link = &self.mesh.links[*link_idx];
                    let new_dist = entry.cost + link.latency;
                    if new_dist < dist.get(neighbor).copied().unwrap_or(f64::INFINITY) {
                        dist.insert(neighbor.clone(), new_dist);
                        prev.insert(neighbor.clone(), entry.node.clone());
                        let mut new_path = entry.path.clone();
                        new_path.push(neighbor.clone());
                        heap.push(DijkstraEntry { node: neighbor.clone(), cost: new_dist, path: new_path });
                    }
                }
            }
        }

        Route::none()
    }

    /// Most-reliable path: maximize product of link reliabilities.
    /// We use -log(reliability) as distance to convert to shortest-path.
    pub fn most_reliable_path(&self, from: &str, to: &str) -> Route {
        let adj = self.mesh.adjacency();
        let mut dist: HashMap<String, f64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        for node in &self.mesh.nodes {
            dist.insert(node.id.clone(), f64::INFINITY);
        }
        dist.insert(from.to_string(), 0.0);
        heap.push(DijkstraEntry {
            node: from.to_string(),
            cost: 0.0,
            path: vec![from.to_string()],
        });

        while let Some(entry) = heap.pop() {
            if entry.node == to {
                let reliability = (-entry.cost).exp();
                let latency: f64 = entry.path.windows(2)
                    .map(|w| self.find_link(&w[0], &w[1]).map(|l| l.latency).unwrap_or(0.0))
                    .sum();
                return Route::new(entry.path, latency, reliability);
            }
            if entry.cost > dist.get(&entry.node).copied().unwrap_or(f64::INFINITY) {
                continue;
            }
            if let Some(neighbors) = adj.get(&entry.node) {
                for (neighbor, link_idx) in neighbors {
                    let link = &self.mesh.links[*link_idx];
                    let rel = link.reliability();
                    let neg_log_rel = if rel > 0.0 { -rel.ln() } else { f64::INFINITY };
                    let new_cost = entry.cost + neg_log_rel;
                    if new_cost < dist.get(neighbor).copied().unwrap_or(f64::INFINITY) {
                        dist.insert(neighbor.clone(), new_cost);
                        let mut new_path = entry.path.clone();
                        new_path.push(neighbor.clone());
                        heap.push(DijkstraEntry { node: neighbor.clone(), cost: new_cost, path: new_path });
                    }
                }
            }
        }

        Route::none()
    }

    /// Widest path: maximize the minimum bandwidth along the path (maximin).
    /// Uses a modified Dijkstra with max-heap on bottleneck bandwidth.
    pub fn widest_path(&self, from: &str, to: &str) -> Route {
        let adj = self.mesh.adjacency();
        let mut best_bottleneck: HashMap<String, f64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        for node in &self.mesh.nodes {
            best_bottleneck.insert(node.id.clone(), f64::NEG_INFINITY);
        }
        best_bottleneck.insert(from.to_string(), f64::INFINITY);
        heap.push(WidestEntry {
            node: from.to_string(),
            bottleneck: f64::INFINITY,
            path: vec![from.to_string()],
        });

        while let Some(entry) = heap.pop() {
            if entry.node == to {
                let latency: f64 = entry.path.windows(2)
                    .map(|w| self.find_link(&w[0], &w[1]).map(|l| l.latency).unwrap_or(0.0))
                    .sum();
                let reliability: f64 = entry.path.windows(2)
                    .map(|w| self.find_link(&w[0], &w[1]).map(|l| l.reliability()).unwrap_or(1.0))
                    .product();
                return Route::new(entry.path, latency, reliability);
            }
            if entry.bottleneck < best_bottleneck.get(&entry.node).copied().unwrap_or(f64::NEG_INFINITY) {
                continue;
            }
            if let Some(neighbors) = adj.get(&entry.node) {
                for (neighbor, link_idx) in neighbors {
                    let link = &self.mesh.links[*link_idx];
                    let new_bottleneck = entry.bottleneck.min(link.bandwidth);
                    if new_bottleneck > best_bottleneck.get(neighbor).copied().unwrap_or(f64::NEG_INFINITY) {
                        best_bottleneck.insert(neighbor.clone(), new_bottleneck);
                        let mut new_path = entry.path.clone();
                        new_path.push(neighbor.clone());
                        heap.push(WidestEntry { node: neighbor.clone(), bottleneck: new_bottleneck, path: new_path });
                    }
                }
            }
        }

        Route::none()
    }

    /// Find all simple paths from `from` to `to` (up to `max_paths`).
    /// Used for multi-path routing and spore broadcast.
    pub fn find_all_paths(&self, from: &str, to: &str, max_paths: usize) -> Vec<Route> {
        let adj = self.mesh.adjacency();
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        visited.insert(from.to_string());
        self.dfs_all(from, to, &adj, &mut visited, &mut vec![from.to_string()], &mut results, max_paths);
        results
    }

    fn dfs_all(
        &self,
        current: &str,
        target: &str,
        adj: &HashMap<String, Vec<(String, usize)>>,
        visited: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
        results: &mut Vec<Route>,
        max_paths: usize,
    ) {
        if results.len() >= max_paths {
            return;
        }
        if current == target {
            let latency: f64 = path.windows(2)
                .map(|w| self.find_link(&w[0], &w[1]).map(|l| l.latency).unwrap_or(0.0))
                .sum();
            let reliability: f64 = path.windows(2)
                .map(|w| self.find_link(&w[0], &w[1]).map(|l| l.reliability()).unwrap_or(1.0))
                .product();
            results.push(Route::new(path.clone(), latency, reliability));
            return;
        }
        if let Some(neighbors) = adj.get(current) {
            for (neighbor, _) in neighbors {
                if !visited.contains(neighbor) {
                    visited.insert(neighbor.clone());
                    path.push(neighbor.clone());
                    self.dfs_all(neighbor, target, adj, visited, path, results, max_paths);
                    path.pop();
                    visited.remove(neighbor);
                }
            }
        }
    }

    fn find_link(&self, a: &str, b: &str) -> Option<&crate::hyphae_link::HyphaeLink> {
        self.mesh.links.iter().find(|l| {
            (l.from == a && l.to == b) || (l.from == b && l.to == a)
        })
    }

    fn build_route<F, G>(&self, path: &[String], latency_fn: F, reliability_fn: G) -> Route
    where
        F: Fn(&crate::hyphae_link::HyphaeLink) -> f64,
        G: Fn(&crate::hyphae_link::HyphaeLink) -> f64,
    {
        let total_latency: f64 = path.windows(2)
            .map(|w| self.find_link(&w[0], &w[1]).map(|l| latency_fn(l)).unwrap_or(0.0))
            .sum();
        let reliability: f64 = path.windows(2)
            .map(|w| self.find_link(&w[0], &w[1]).map(|l| reliability_fn(l)).unwrap_or(1.0))
            .product();
        Route::new(path.to_vec(), total_latency, reliability)
    }
}

/// Entry for widest-path Dijkstra (max-heap on bottleneck).
#[derive(Debug, Clone)]
struct WidestEntry {
    node: String,
    bottleneck: f64,
    path: Vec<String>,
}

impl PartialEq for WidestEntry {
    fn eq(&self, other: &Self) -> bool { self.bottleneck == other.bottleneck }
}
impl Eq for WidestEntry {}
impl PartialOrd for WidestEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for WidestEntry {
    fn cmp(&self, other: &Self) -> Ordering { self.bottleneck.partial_cmp(&other.bottleneck).unwrap_or(Ordering::Equal) }
}
