//! Individual hyphae link: bandwidth, latency, error profile.
//!
//! Each fungal hypha has physical characteristics — how wide it is (bandwidth),
//! how far it stretches (latency), and how error-prone its transmission is.
//! Shannon's theorem gives us the theoretical capacity of each link.

use serde::{Deserialize, Serialize};

/// Error profile for a hyphae link.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorProfile {
    /// Random bit errors at a steady rate.
    Random { ber: f64 },
    /// Burst errors: errors come in clumps of given length at given frequency.
    Burst { burst_length: usize, burst_frequency: f64 },
    /// Mixed: both random and burst components.
    Mixed { ber: f64, burst_length: usize, burst_frequency: f64 },
}

/// A single hyphae link between two mesh nodes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HyphaeLink {
    pub from: String,
    pub to: String,
    /// Bandwidth in bits per second.
    pub bandwidth: f64,
    /// Base error rate (probability of bit error).
    pub error_rate: f64,
    /// Shannon capacity in bits per second.
    pub capacity: f64,
    /// Latency in milliseconds.
    pub latency: f64,
    /// Error profile for detailed modeling.
    pub error_profile: ErrorProfile,
}

impl HyphaeLink {
    /// Create a new link and compute Shannon capacity.
    ///
    /// Shannon capacity: C = B * log2(1 + S/N)
    /// We model S/N as (1 - error_rate) / error_rate (simplified SNR).
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        bandwidth: f64,
        error_rate: f64,
    ) -> Self {
        let capacity = Self::shannon_capacity(bandwidth, error_rate);
        let latency = 1.0; // default 1ms
        Self {
            from: from.into(),
            to: to.into(),
            bandwidth,
            error_rate,
            capacity,
            latency,
            error_profile: ErrorProfile::Random { ber: error_rate },
        }
    }

    /// Create with full parameters.
    pub fn with_details(
        from: impl Into<String>,
        to: impl Into<String>,
        bandwidth: f64,
        error_rate: f64,
        latency: f64,
        error_profile: ErrorProfile,
    ) -> Self {
        let capacity = Self::shannon_capacity(bandwidth, error_rate);
        Self {
            from: from.into(),
            to: to.into(),
            bandwidth,
            error_rate,
            capacity,
            latency,
            error_profile,
        }
    }

    /// Compute Shannon capacity: C = B * log2(1 + SNR).
    ///
    /// We use a simplified model where SNR = (1 - error_rate) / error_rate.
    /// For error_rate = 0, capacity equals bandwidth.
    /// For error_rate >= 1, capacity is 0.
    pub fn shannon_capacity(bandwidth: f64, error_rate: f64) -> f64 {
        if error_rate <= 0.0 {
            bandwidth
        } else if error_rate >= 1.0 {
            0.0
        } else {
            let snr = (1.0 - error_rate) / error_rate;
            bandwidth * (1.0 + snr).log2()
        }
    }

    /// Reliability of this link: probability of successful transmission.
    /// Modeled as (1 - error_rate) for simplicity.
    pub fn reliability(&self) -> f64 {
        (1.0 - self.error_rate).max(0.0).min(1.0)
    }

    /// Unique identifier for this link (undirected).
    pub fn id(&self) -> String {
        let (a, b) = if self.from < self.to {
            (&self.from, &self.to)
        } else {
            (&self.to, &self.from)
        };
        format!("{}-{}", a, b)
    }
}
