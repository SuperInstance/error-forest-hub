//! Hub relay: error-correcting relay nodes.
//!
//! Hub nodes in the mycorrhizal mesh act like major fungal junctions —
//! they receive messages, apply phyto-code error correction, and relay
//! them onward. Each hop reduces the residual error rate, so messages
//! that traverse multiple hubs become progressively more reliable.

use serde::{Deserialize, Serialize};

/// A single hop through a hub relay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelayHop {
    pub hub_id: String,
    pub incoming_error_rate: f64,
    pub outgoing_error_rate: f64,
    /// Phyto-code correction factor applied at this hop.
    pub correction_factor: f64,
}

impl RelayHop {
    pub fn new(hub_id: impl Into<String>, incoming_error: f64, correction: f64) -> Self {
        let outgoing = incoming_error * (1.0 - correction);
        Self {
            hub_id: hub_id.into(),
            incoming_error_rate: incoming_error,
            outgoing_error_rate: outgoing,
            correction_factor: correction,
        }
    }
}

/// Result of relaying a message through a series of hub nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelayResult {
    pub path: Vec<String>,
    pub hops: Vec<RelayHop>,
    /// Cumulative error rate after all corrections.
    pub final_error_rate: f64,
    /// Overall reliability = 1 - final_error_rate.
    pub reliability: f64,
    /// Total latency across all hops.
    pub total_latency: f64,
    /// Whether the message is considered delivered successfully.
    pub delivered: bool,
}

impl RelayResult {
    pub fn new(path: Vec<String>, hops: Vec<RelayHop>, total_latency: f64) -> Self {
        let final_error_rate = hops.last()
            .map(|h| h.outgoing_error_rate)
            .unwrap_or(0.0);
        let reliability = 1.0 - final_error_rate;
        // Delivered if reliability > 50% (configurable threshold)
        let delivered = reliability > 0.5;
        Self { path, hops, final_error_rate, reliability, total_latency, delivered }
    }
}

/// Hub relay that applies error correction at each hop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HubRelay {
    /// Default phyto-code correction factor per hop (0.0 to 1.0).
    pub correction_factor: f64,
    /// Per-hub correction overrides.
    pub hub_corrections: std::collections::HashMap<String, f64>,
}

impl HubRelay {
    pub fn new(correction_factor: f64) -> Self {
        Self {
            correction_factor,
            hub_corrections: std::collections::HashMap::new(),
        }
    }

    /// Set a custom correction factor for a specific hub.
    pub fn with_hub_correction(mut self, hub_id: impl Into<String>, factor: f64) -> Self {
        self.hub_corrections.insert(hub_id.into(), factor);
        self
    }

    /// Relay a message along a path of nodes through the mesh.
    ///
    /// At each hop that passes through a hub node, error correction is applied.
    /// The error rate compounds across links and gets corrected at hubs.
    pub fn relay_along_path(
        &self,
        path: &[String],
        mesh: &crate::mesh::Mesh,
        initial_error_rate: f64,
    ) -> RelayResult {
        let mut current_error = initial_error_rate;
        let mut hops = Vec::new();
        let mut total_latency = 0.0;

        for i in 0..path.len().saturating_sub(1) {
            let node_id = &path[i];
            let link = mesh.links.iter().find(|l| {
                (l.from == path[i] && l.to == path[i + 1])
                    || (l.to == path[i] && l.from == path[i + 1])
            });

            // Add link error contribution
            if let Some(link) = link {
                // Error compounds: p_combined = 1 - (1-p1)(1-p2)
                let link_rel = 1.0 - link.error_rate;
                let current_rel = 1.0 - current_error;
                current_error = 1.0 - (current_rel * link_rel);
                total_latency += link.latency;
            }

            // If this node is a hub, apply correction
            let node = mesh.get_node(node_id);
            if let Some(node) = node {
                if node.is_hub {
                    let correction = self.hub_corrections
                        .get(&node.id)
                        .copied()
                        .unwrap_or(self.correction_factor);
                    let incoming = current_error;
                    current_error = current_error * (1.0 - correction);
                    hops.push(RelayHop::new(&node.id, incoming, correction));
                }
            }
        }

        // Check last node too (it might be a hub destination)
        if let Some(last_id) = path.last() {
            if let Some(node) = mesh.get_node(last_id) {
                if node.is_hub {
                    let correction = self.hub_corrections
                        .get(&node.id)
                        .copied()
                        .unwrap_or(self.correction_factor);
                    let incoming = current_error;
                    current_error = current_error * (1.0 - correction);
                    hops.push(RelayHop::new(&node.id, incoming, correction));
                }
            }
        }

        RelayResult::new(path.to_vec(), hops, total_latency)
    }

    /// Cumulative error correction: compute final error after multiple hub hops.
    /// Each hub multiplies error by (1 - correction_factor).
    pub fn cumulative_error_correction(&self, initial_error: f64, num_hubs: usize) -> f64 {
        let factor = 1.0 - self.correction_factor;
        initial_error * factor.powi(num_hubs as i32)
    }
}

impl Default for HubRelay {
    fn default() -> Self {
        Self::new(0.5)
    }
}
