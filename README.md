# 🍄 Error Forest Hub

**A distributed error-correction hub network — a mycorrhizal mesh for resilient message delivery.**

```
┌─────────────────────────────────────────────────────────────────────┐
│                                                                     │
│                    🌳  ERROR FOREST HUB  🌳                         │
│                                                                     │
│   Trees are fragile. One broken branch and the message is lost.     │
│   But underground, a fungal network connects them all —             │
│   a mesh of hyphae that reroutes around damage,                     │
│   corrects errors at every hub, and delivers even when              │
│   half the network is on fire.                                      │
│                                                                     │
│   This is that network.                                             │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## 🧬 The Metaphor

In real forests, trees don't stand alone. Underground, **mycorrhizal fungi** form vast networks connecting trees through hair-thin **hyphae**. These fungal highways transport nutrients, chemical signals, and warnings across the forest floor.

Key properties:
- **Mother trees** (hub nodes) act as relay stations with extra processing power
- **Hyphae links** vary in bandwidth and reliability — some are thick and fast, others thin and lossy
- **Redundant paths** mean a severed hypha doesn't stop the flow — nutrients reroute
- **Spore dispersal** broadcasts messages in all directions, overwhelming noise through sheer diversity

**Error Forest Hub** models this biological architecture as a distributed error-correction network.

## 🏗️ Architecture

```
  TREE TOPOLOGY (fragile)           MESH TOPOLOGY (resilient)

      A                                 A ─────────── D
      │                                / \ ╲         │
      │                               /   \  ╲       │
      B                              B─────C───E      │
      │                              │     │   │     │
      │                              └─────┘   └─────┘
      C
      │                            • No single point of failure
      │                            • k-resilience ≥ 1
      D                            • Multiple delivery paths

  ❌ Remove A-B → disconnected     ✅ Remove any link → still connected
  ❌ k-resilience = 0              ✅ k-resilience ≥ 1
```

### Module Map

| Module | Purpose | Biological Analog |
|--------|---------|-------------------|
| `mesh` | Network topology — nodes & links | The underground fungal network |
| `hyphae_link` | Individual link characteristics | A single fungal hypha strand |
| `hub_relay` | Error-correcting relay nodes | Mother trees / hub junctions |
| `mesh_routing` | Path-finding algorithms | How nutrients find optimal routes |
| `resilience` | Failure tolerance analysis | How the forest survives damage |
| `spore_broadcast` | Multi-path broadcast delivery | Fungal spore dispersal |

## 🚀 Quick Start

```rust
use error_forest_hub::*;

// 1. Build a mesh
let mut mesh = mesh::Mesh::new();
mesh.add_node(mesh::MeshNode::new("Oak", true, (0.0, 0.0)));
mesh.add_node(mesh::MeshNode::new("Pine", true, (1.0, 0.0)));
mesh.add_node(mesh::MeshNode::new("Birch", false, (0.5, 0.87)));
mesh.add_node(mesh::MeshNode::new("Maple", true, (1.5, 0.87)));

mesh.add_link(hyphae_link::HyphaeLink::new("Oak", "Pine", 100.0, 0.01));
mesh.add_link(hyphae_link::HyphaeLink::new("Oak", "Birch", 80.0, 0.02));
mesh.add_link(hyphae_link::HyphaeLink::new("Pine", "Birch", 60.0, 0.01));
mesh.add_link(hyphae_link::HyphaeLink::new("Pine", "Maple", 90.0, 0.015));
mesh.add_link(hyphae_link::HyphaeLink::new("Birch", "Maple", 70.0, 0.02));

// 2. Check resilience
let analyzer = resilience::ResilienceAnalyzer::new(&mesh);
let report = analyzer.analyze();
println!("k-resilience: {}", report.k_resilience); // 1 — survives any single failure
println!("Bridges: {:?}", report.critical_links);   // none!

// 3. Find the best route
let router = mesh_routing::MeshRouter::new(&mesh);
let route = router.route(
    "Oak", "Maple",
    &mesh_routing::RoutingAlgorithm::MostReliable,
);
println!("Path: {:?}", route.path);
println!("Reliability: {:.2}%", route.reliability * 100.0);

// 4. Relay with error correction
let relay = hub_relay::HubRelay::new(0.6); // 60% correction per hub
let result = relay.relay_along_path(&route.path, &mesh, 0.05);
println!("Final error rate: {:.4}", result.final_error_rate);
println!("Delivered: {}", result.delivered);

// 5. Spore broadcast for critical messages
let mut broadcast = spore_broadcast::SporeBroadcast::new("Oak", vec!["Maple".into()], 5)
    .with_delivery_threshold(2); // Need 2 successful paths
broadcast.plan(&mesh);
let delivery = broadcast.simulate_delivery(&[("Oak", "Pine")]); // One link fails
println!("Still delivered: {}", delivery.delivered); // true!
```

## 🧭 Routing Algorithms

Three routing strategies, each optimizing for a different goal:

### Shortest Path
Minimizes total latency. Classic Dijkstra.

```
Oak ──[1ms]── Pine ──[1ms]── Maple    → 2ms, reliability 0.975
```

### Most Reliable Path
Maximizes the product of link reliabilities (minimizes -log reliability).

```
Oak ──[1ms]── Birch ──[1ms]── Pine ──[1ms]── Maple
 reliability: 0.98 × 0.99 × 0.985 = 0.955

vs direct: Oak → Pine → Maple = 0.99 × 0.985 = 0.975  ← better!
```

### Widest Path (Maximin Bandwidth)
Maximizes the minimum bandwidth along the path. Critical for throughput-sensitive messages.

```
Path A: min(100, 70) = 70 Mbps bottleneck
Path B: min(90, 85)  = 85 Mbps bottleneck  ← wider!
```

## 🛡️ Resilience Analysis

```rust
let analyzer = ResilienceAnalyzer::new(&mesh);
let report = analyzer.analyze();

// k-resilience: survives k simultaneous link failures
assert!(report.k_resilience >= 1);

// Bridge detection: links whose removal disconnects the mesh
assert!(report.critical_links.is_empty()); // No bridges in a mesh!

// Compare with tree topology
let (mesh_k, tree_k) = analyzer.compare_with_tree();
// mesh_k > 0, tree_k == 0
```

### Why Mesh > Tree

```
Scenario: 3 link failures in a 7-link network

Tree:  A─B─C─D─E─F─G
       ╳ ╳ ╳        (3 failures = game over)
       → Disconnected after failure #1

Mesh:  A─B─C
       │╲│╲│         (3 failures... still standing)
       D─E─F
       │ │ │
       G─H─I
       → Survives! k-resilience = 2
```

## 🌬️ Spore Broadcast

For messages that absolutely must arrive, use spore broadcast:

```rust
let mut broadcast = SporeBroadcast::new("source", vec!["target".into()], 10)
    .with_delivery_threshold(2); // 2 of N paths must succeed

broadcast.plan(&mesh);

// Simulate burst errors knocking out several links
let result = broadcast.simulate_delivery(&[
    ("A", "B"),  // link down
    ("C", "D"),  // link down
]);

assert!(result.delivered); // Still delivered via alternate paths!
```

The `delivery_threshold` parameter lets you tune the tradeoff between:
- **Reliability** (higher threshold = more paths must succeed)
- **Overhead** (higher threshold = more redundant transmission)

## 🔬 Shannon Capacity

Each hyphae link computes its theoretical Shannon capacity:

```
C = B × log₂(1 + SNR)

where SNR = (1 - error_rate) / error_rate
```

| Error Rate | SNR | Capacity (B=100) |
|-----------|-----|-------------------|
| 0.001 | 999 | 996 Mbps |
| 0.01 | 99 | 664 Mbps |
| 0.1 | 9 | 345 Mbps |
| 0.5 | 1 | 100 Mbps |
| 0.9 | 0.11 | 36 Mbps |

## 📊 Hub Relay Error Correction

Hub nodes apply phyto-code error correction at each hop, exponentially reducing errors:

```
Initial error rate: 10%
Correction factor: 60% per hub

After hub 1: 10% × 0.4 = 4%
After hub 2: 4% × 0.4 = 1.6%
After hub 3: 1.6% × 0.4 = 0.64%
After hub 4: 0.64% × 0.4 = 0.256%
```

Custom correction per hub:

```rust
let relay = HubRelay::new(0.5)
    .with_hub_correction("MotherOak", 0.9)   // Super hub
    .with_hub_correction("WeakBirch", 0.2);  // Struggling hub
```

## 🧪 Testing

```bash
cargo test
```

54 tests covering:
- Mesh construction and connectivity
- Hub node identification
- Shannon capacity computation
- All three routing algorithms
- k-resilience and bridge detection
- Tree vs mesh resilience comparison
- Spore broadcast multi-path delivery
- Full pipeline: mesh → route → relay → correct → deliver
- Cumulative error correction
- Serde serialization roundtrips

## 📦 Core Types

```rust
// Network topology
struct Mesh { nodes: Vec<MeshNode>, links: Vec<HyphaeLink> }
struct MeshNode { id: String, is_hub: bool, position: (f64, f64) }
struct HyphaeLink { from: String, to: String, bandwidth: f64, error_rate: f64, capacity: f64 }

// Routing
struct Route { path: Vec<String>, total_latency: f64, reliability: f64, hops: usize }
enum RoutingAlgorithm { ShortestPath, MostReliable, WidestPath }

// Resilience
struct ResilienceReport { k_resilience: usize, critical_links: Vec<String>, bottleneck_node: Option<String> }

// Broadcast
struct SporeBroadcast { source: String, targets: Vec<String>, fanout: usize, paths: Vec<Route> }

// Relay
struct RelayResult { path: Vec<String>, hops: Vec<RelayHop>, final_error_rate: f64, reliability: f64, delivered: bool }
```

All types implement `Serialize` and `Deserialize` from serde.

## 🔧 Installation

```toml
[dependencies]
error-forest-hub = "0.1"
```

## 📜 License

MIT

---

*In every forest, the trees you see are only half the story. The real network is underground.*
