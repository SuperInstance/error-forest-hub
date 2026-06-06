//! Spore broadcast: multi-path message delivery inspired by fungal spore dispersal.
//!
//! In nature, fungi release spores in multiple directions. Some spores fail,
//! but the species survives because enough reach fertile ground. Similarly,
//! a spore broadcast sends messages along multiple paths, and delivery
//! succeeds if enough paths get through — providing burst-error tolerance.

use serde::{Deserialize, Serialize};

/// A spore broadcast: message sent along multiple paths to multiple targets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SporeBroadcast {
    pub source: String,
    pub targets: Vec<String>,
    /// Maximum number of paths to explore per target.
    pub fanout: usize,
    /// Computed paths to all targets.
    pub paths: Vec<crate::mesh_routing::Route>,
    /// Minimum number of paths that must succeed for delivery.
    pub delivery_threshold: usize,
}

impl SporeBroadcast {
    /// Create a new spore broadcast plan.
    pub fn new(
        source: impl Into<String>,
        targets: Vec<String>,
        fanout: usize,
    ) -> Self {
        Self {
            source: source.into(),
            targets,
            fanout,
            paths: Vec::new(),
            delivery_threshold: 1,
        }
    }

    /// Plan the broadcast: find multiple paths to each target.
    pub fn plan(&mut self, mesh: &crate::mesh::Mesh) {
        let router = crate::mesh_routing::MeshRouter::new(mesh);
        self.paths.clear();

        for target in &self.targets {
            let all_paths = router.find_all_paths(&self.source, target, self.fanout);
            self.paths.extend(all_paths);
        }
    }

    /// Simulate broadcast delivery given which links fail.
    /// Returns (delivered, successful_paths, failed_paths).
    pub fn simulate_delivery(
        &self,
        failed_links: &[(&str, &str)],
    ) -> BroadcastResult {
        let failed_set: std::collections::HashSet<String> = failed_links
            .iter()
            .map(|(a, b)| {
                if *a < *b { format!("{}-{}", a, b) } else { format!("{}-{}", b, a) }
            })
            .collect();

        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for route in &self.paths {
            let mut path_ok = true;
            for window in route.path.windows(2) {
                let a = &window[0];
                let b = &window[1];
                let link_id = if a < b { format!("{}-{}", a, b) } else { format!("{}-{}", b, a) };
                if failed_set.contains(&link_id) {
                    path_ok = false;
                    break;
                }
            }
            if path_ok {
                successful.push(route.clone());
            } else {
                failed.push(route.clone());
            }
        }

        let delivered = successful.len() >= self.delivery_threshold;
        BroadcastResult {
            delivered,
            successful_paths: successful.len(),
            failed_paths: failed.len(),
            total_paths: self.paths.len(),
        }
    }

    /// Per-target delivery check: does each target have enough surviving paths?
    pub fn per_target_delivery(
        &self,
        failed_links: &[(&str, &str)],
    ) -> Vec<TargetDelivery> {
        let failed_set: std::collections::HashSet<String> = failed_links
            .iter()
            .map(|(a, b)| {
                if *a < *b { format!("{}-{}", a, b) } else { format!("{}-{}", b, a) }
            })
            .collect();

        self.targets.iter().map(|target| {
            let target_paths: Vec<&crate::mesh_routing::Route> = self.paths.iter()
                .filter(|r| r.path.last().map(|p| p == target).unwrap_or(false))
                .collect();

            let successful = target_paths.iter().filter(|route| {
                for window in route.path.windows(2) {
                    let a = &window[0];
                    let b = &window[1];
                    let link_id = if a < b { format!("{}-{}", a, b) } else { format!("{}-{}", b, a) };
                    if failed_set.contains(&link_id) {
                        return false;
                    }
                }
                true
            }).count();

            TargetDelivery {
                target: target.clone(),
                total_paths: target_paths.len(),
                successful_paths: successful,
                delivered: successful >= self.delivery_threshold,
            }
        }).collect()
    }

    /// Set the delivery threshold (minimum successful paths needed).
    pub fn with_delivery_threshold(mut self, threshold: usize) -> Self {
        self.delivery_threshold = threshold;
        self
    }
}

/// Result of a broadcast simulation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BroadcastResult {
    pub delivered: bool,
    pub successful_paths: usize,
    pub failed_paths: usize,
    pub total_paths: usize,
}

/// Per-target delivery result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TargetDelivery {
    pub target: String,
    pub total_paths: usize,
    pub successful_paths: usize,
    pub delivered: bool,
}
