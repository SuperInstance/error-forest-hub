//! Tutorial: error-forest-hub — Mycorrhizal mesh error-correction network
//!
//! Hub relays, hyphae links with Shannon capacity, mesh routing, spore broadcast.

use error_forest_hub::{
    HubRelay, HyphaeLink,
    Mesh, MeshNode,
};

fn main() {
    println!("=== Error Forest Hub Tutorial ===\n");

    // Part 1: Build mesh topology
    println!("Part 1: Mesh network");
    let mut mesh = Mesh::new();
    mesh.add_node(MeshNode::new("A", true, (0.0, 0.0)));
    mesh.add_node(MeshNode::new("B", false, (1.0, 0.0)));
    mesh.add_node(MeshNode::new("C", false, (0.5, 1.0)));
    mesh.add_node(MeshNode::new("D", true, (1.5, 1.0)));
    println!("  4 nodes (2 hubs, 2 roots)");
    println!();

    // Part 2: Hyphae links
    println!("Part 2: Hyphae links (fungal channels)");
    mesh.add_link(HyphaeLink::new("A", "B", 100.0, 0.05));
    mesh.add_link(HyphaeLink::new("B", "C", 80.0, 0.08));
    mesh.add_link(HyphaeLink::new("A", "C", 90.0, 0.03));
    mesh.add_link(HyphaeLink::new("C", "D", 70.0, 0.10));
    let capacity = HyphaeLink::shannon_capacity(100.0, 0.05);
    println!("  4 links added");
    println!("  Shannon capacity (BW=100, err=0.05): {:.2}", capacity);
    println!();

    // Part 3: Hub relay
    println!("Part 3: Hub relay (error correction)");
    let relay = HubRelay::new(0.1)
        .with_hub_correction("A", 0.3)
        .with_hub_correction("D", 0.2);
    let correction = relay.cumulative_error_correction(1.0, 3);
    println!("  Error 1.0 after 3 hubs: {:.4}", correction);
    println!();

    // Part 4: Relay along path
    println!("Part 4: Relay along path");
    let path = vec!["B".into(), "A".into(), "C".into(), "D".into()];
    let result = relay.relay_along_path(&path, &mesh, 0.5);
    println!("  Path: {:?}", result.path);
    println!("  Hops: {} relays", result.hops.len());
    println!("  Total latency: {:.4}", result.total_latency);
}
