#[cfg(test)]
mod tests {
    use error_forest_hub::*;
    use error_forest_hub::mesh::{Mesh, MeshNode};
    use error_forest_hub::hyphae_link::{HyphaeLink, ErrorProfile};
    use error_forest_hub::mesh_routing::{MeshRouter, RoutingAlgorithm, Route};
    use error_forest_hub::hub_relay::{HubRelay, RelayHop, RelayResult};
    use error_forest_hub::resilience::{ResilienceAnalyzer, ResilienceReport, LinkFailure};
    use error_forest_hub::spore_broadcast::{SporeBroadcast, BroadcastResult};

    // Helper: build a simple mesh for testing
    fn triangle_mesh() -> Mesh {
        Mesh::new()
            .with_nodes(vec![
                MeshNode::new("A", true, (0.0, 0.0)),
                MeshNode::new("B", true, (1.0, 0.0)),
                MeshNode::new("C", false, (0.5, 0.866)),
            ])
            .with_links(vec![
                HyphaeLink::new("A", "B", 100.0, 0.01),
                HyphaeLink::new("B", "C", 80.0, 0.02),
                HyphaeLink::new("A", "C", 60.0, 0.03),
            ])
    }

    fn tree_mesh() -> Mesh {
        // A tree: A-B, B-C, C-D (no cycles)
        Mesh::new()
            .with_nodes(vec![
                MeshNode::new("A", true, (0.0, 0.0)),
                MeshNode::new("B", true, (1.0, 0.0)),
                MeshNode::new("C", false, (2.0, 0.0)),
                MeshNode::new("D", false, (3.0, 0.0)),
            ])
            .with_links(vec![
                HyphaeLink::new("A", "B", 100.0, 0.01),
                HyphaeLink::new("B", "C", 80.0, 0.02),
                HyphaeLink::new("C", "D", 60.0, 0.03),
            ])
    }

    fn rich_mesh() -> Mesh {
        // 5-node mesh with multiple paths
        Mesh::new()
            .with_nodes(vec![
                MeshNode::new("S", true, (0.0, 0.0)),   // source hub
                MeshNode::new("H1", true, (1.0, 1.0)),  // hub
                MeshNode::new("H2", true, (1.0, -1.0)), // hub
                MeshNode::new("L1", false, (2.0, 1.0)), // leaf
                MeshNode::new("D", true, (3.0, 0.0)),   // destination hub
            ])
            .with_links(vec![
                HyphaeLink::new("S", "H1", 100.0, 0.01),
                HyphaeLink::new("S", "H2", 90.0, 0.02),
                HyphaeLink::new("H1", "L1", 80.0, 0.01),
                HyphaeLink::new("H1", "D", 70.0, 0.02),
                HyphaeLink::new("H2", "D", 85.0, 0.015),
                HyphaeLink::new("L1", "D", 60.0, 0.03),
                HyphaeLink::new("H1", "H2", 95.0, 0.005),
            ])
    }

    // === MESH CONSTRUCTION ===

    #[test]
    fn test_mesh_construction() {
        let mesh = triangle_mesh();
        assert_eq!(mesh.nodes.len(), 3);
        assert_eq!(mesh.links.len(), 3);
    }

    #[test]
    fn test_mesh_is_connected() {
        let mesh = triangle_mesh();
        assert!(mesh.is_connected());
    }

    #[test]
    fn test_disconnected_mesh() {
        let mesh = Mesh::new()
            .with_nodes(vec![
                MeshNode::new("A", false, (0.0, 0.0)),
                MeshNode::new("B", false, (5.0, 5.0)),
            ]);
        assert!(!mesh.is_connected());
    }

    #[test]
    fn test_empty_mesh_is_connected() {
        assert!(Mesh::new().is_connected());
    }

    #[test]
    fn test_mesh_connectivity_after_removal() {
        let mesh = triangle_mesh();
        // Triangle is still connected after removing any single link
        let modified = mesh.without_link_by_endpoints("A", "B");
        assert!(modified.is_connected());
    }

    // === HUB NODES ===

    #[test]
    fn test_hub_nodes_identified() {
        let mesh = triangle_mesh();
        let hubs = mesh.hub_nodes();
        assert_eq!(hubs.len(), 2);
        let hub_ids: Vec<&str> = hubs.iter().map(|h| h.id.as_str()).collect();
        assert!(hub_ids.contains(&"A"));
        assert!(hub_ids.contains(&"B"));
        assert!(!hub_ids.contains(&"C"));
    }

    #[test]
    fn test_rich_mesh_hubs() {
        let mesh = rich_mesh();
        assert_eq!(mesh.hub_nodes().len(), 4); // S, H1, H2, D
    }

    // === HYPHAE LINK ===

    #[test]
    fn test_shannon_capacity_perfect_channel() {
        // Perfect channel: error_rate = 0 → capacity = bandwidth
        let cap = HyphaeLink::shannon_capacity(100.0, 0.0);
        assert_eq!(cap, 100.0);
    }

    #[test]
    fn test_shannon_capacity_noisy_channel() {
        // Noisy channel: capacity > 0 but finite
        let cap = HyphaeLink::shannon_capacity(100.0, 0.5);
        // C = B * log2(1 + 1) = B * 1 = B at error_rate=0.5
        assert!(cap > 0.0);
        // With higher error rate, capacity should be lower
        let cap_worse = HyphaeLink::shannon_capacity(100.0, 0.9);
        assert!(cap_worse < cap);
    }

    #[test]
    fn test_shannon_capacity_dead_channel() {
        // Completely dead: error_rate >= 1 → capacity = 0
        let cap = HyphaeLink::shannon_capacity(100.0, 1.0);
        assert_eq!(cap, 0.0);
    }

    #[test]
    fn test_shannon_capacity_low_error() {
        // Low error: capacity should be very high (log2(1/0.001) ≈ 10)
        let cap = HyphaeLink::shannon_capacity(1000.0, 0.001);
        assert!(cap > 9000.0); // Much higher than bandwidth due to high SNR
    }

    #[test]
    fn test_link_reliability() {
        let link = HyphaeLink::new("A", "B", 100.0, 0.05);
        assert!((link.reliability() - 0.95).abs() < 1e-10);
    }

    #[test]
    fn test_link_id_undirected() {
        let link1 = HyphaeLink::new("A", "B", 100.0, 0.01);
        let link2 = HyphaeLink::new("B", "A", 100.0, 0.01);
        assert_eq!(link1.id(), link2.id());
    }

    #[test]
    fn test_link_error_profile() {
        let link = HyphaeLink::with_details(
            "A", "B", 100.0, 0.02, 5.0,
            ErrorProfile::Burst { burst_length: 4, burst_frequency: 0.1 },
        );
        assert_eq!(link.latency, 5.0);
        assert!(matches!(link.error_profile, ErrorProfile::Burst { .. }));
    }

    // === ROUTING ===

    #[test]
    fn test_shortest_path_triangle() {
        let mesh = triangle_mesh();
        let router = MeshRouter::new(&mesh);
        let route = router.shortest_path("A", "C");
        assert!(route.is_valid());
        assert_eq!(route.path.first().unwrap(), "A");
        assert_eq!(route.path.last().unwrap(), "C");
        // Direct A→C has latency 1.0; A→B→C has latency 2.0
        assert!(route.total_latency <= 2.0);
    }

    #[test]
    fn test_shortest_path_no_route() {
        let mesh = Mesh::new()
            .with_nodes(vec![
                MeshNode::new("A", false, (0.0, 0.0)),
                MeshNode::new("B", false, (5.0, 5.0)),
            ]);
        let router = MeshRouter::new(&mesh);
        let route = router.shortest_path("A", "B");
        assert!(!route.is_valid());
    }

    #[test]
    fn test_most_reliable_path() {
        let mesh = triangle_mesh();
        let router = MeshRouter::new(&mesh);
        let route = router.most_reliable_path("A", "C");
        assert!(route.is_valid());
        // A→B→C reliability = 0.99 * 0.98 = 0.9702
        // A→C reliability = 0.97
        // So most reliable should be A→B→C
        assert!(route.reliability > 0.97);
        // Check it's NOT the direct path (which has reliability 0.97)
        assert!(route.path.len() > 2);
    }

    #[test]
    fn test_widest_path() {
        let mesh = rich_mesh();
        let router = MeshRouter::new(&mesh);
        let route = router.widest_path("S", "D");
        assert!(route.is_valid());
        // The widest path maximizes minimum bandwidth.
        // S→H1→D: min(100, 70) = 70
        // S→H2→D: min(90, 85) = 85
        // S→H1→H2→D: min(100, 95, 85) = 85
        // So widest should give bottleneck >= 85
    }

    #[test]
    fn test_routing_algorithm_enum() {
        let mesh = triangle_mesh();
        let router = MeshRouter::new(&mesh);

        let sp = router.route("A", "C", &RoutingAlgorithm::ShortestPath);
        let mr = router.route("A", "C", &RoutingAlgorithm::MostReliable);
        let wp = router.route("A", "C", &RoutingAlgorithm::WidestPath);

        assert!(sp.is_valid());
        assert!(mr.is_valid());
        assert!(wp.is_valid());
    }

    // === RESILIENCE ===

    #[test]
    fn test_mesh_survives_single_link_failure() {
        let mesh = triangle_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        // Triangle has no bridges, should survive any single removal
        for link in &mesh.links {
            let modified = mesh.without_link_by_endpoints(&link.from, &link.to);
            assert!(modified.is_connected(), "Mesh should survive removal of {}-{}", link.from, link.to);
        }
    }

    #[test]
    fn test_tree_fails_on_single_link() {
        let tree = tree_mesh();
        // A tree has all bridges: removing any link disconnects it
        for link in &tree.links {
            let modified = tree.without_link_by_endpoints(&link.from, &link.to);
            assert!(!modified.is_connected(), "Tree should fail on removal of {}-{}", link.from, link.to);
        }
    }

    #[test]
    fn test_k_resilience_triangle() {
        let mesh = triangle_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        let report = analyzer.analyze();
        // Triangle: 3 edges, edge connectivity = 2, so k = 1
        assert_eq!(report.k_resilience, 1);
    }

    #[test]
    fn test_k_resilience_tree() {
        let tree = tree_mesh();
        let analyzer = ResilienceAnalyzer::new(&tree);
        let report = analyzer.analyze();
        assert_eq!(report.k_resilience, 0);
    }

    #[test]
    fn test_critical_links_bridges() {
        let tree = tree_mesh();
        let analyzer = ResilienceAnalyzer::new(&tree);
        let bridges = analyzer.find_bridges();
        // Every link in a tree is a bridge
        assert_eq!(bridges.len(), 3);
    }

    #[test]
    fn test_no_bridges_in_triangle() {
        let mesh = triangle_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        let bridges = analyzer.find_bridges();
        assert!(bridges.is_empty(), "Triangle has no bridges");
    }

    #[test]
    fn test_resilience_report() {
        let mesh = rich_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        let report = analyzer.analyze();
        assert!(report.is_connected);
        assert_eq!(report.total_nodes, 5);
        assert_eq!(report.total_links, 7);
        assert!(report.k_resilience >= 1);
    }

    #[test]
    fn test_survives_failures() {
        let mesh = rich_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        // Rich mesh should survive single link failure
        let failure = LinkFailure::new("S", "H1");
        assert!(analyzer.survives_failures(&[failure]));
    }

    #[test]
    fn test_compare_with_tree() {
        let mesh = rich_mesh();
        let analyzer = ResilienceAnalyzer::new(&mesh);
        let (mesh_k, tree_k) = analyzer.compare_with_tree();
        assert!(mesh_k > 0);
        assert_eq!(tree_k, 0);
    }

    #[test]
    fn test_bottleneck_node() {
        let tree = tree_mesh();
        let analyzer = ResilienceAnalyzer::new(&tree);
        let report = analyzer.analyze();
        // In a line tree, B and C are both cut-vertices
        assert!(report.bottleneck_node.is_some());
    }

    // === HUB RELAY ===

    #[test]
    fn test_relay_along_path() {
        let mesh = rich_mesh();
        let relay = HubRelay::new(0.5);
        let result = relay.relay_along_path(&["S".into(), "H1".into(), "D".into()], &mesh, 0.0);
        assert!(result.reliability > 0.0);
        assert!(result.hops.len() >= 2); // S and H1 and D are hubs
    }

    #[test]
    fn test_cumulative_error_correction() {
        let relay = HubRelay::new(0.5);
        // After 3 hub hops: error * 0.5^3 = error * 0.125
        let final_error = relay.cumulative_error_correction(0.1, 3);
        assert!((final_error - 0.0125).abs() < 1e-10);
    }

    #[test]
    fn test_relay_hop_correction() {
        let hop = RelayHop::new("H1", 0.1, 0.5);
        assert!((hop.outgoing_error_rate - 0.05).abs() < 1e-10);
    }

    #[test]
    fn test_relay_with_custom_hub_correction() {
        let relay = HubRelay::new(0.3)
            .with_hub_correction("H1", 0.9);
        let mesh = rich_mesh();
        let result = relay.relay_along_path(&["S".into(), "H1".into(), "D".into()], &mesh, 0.0);
        assert!(result.reliability > 0.5);
    }

    // === SPORE BROADCAST ===

    #[test]
    fn test_spore_broadcast_reaches_targets() {
        let mesh = rich_mesh();
        let mut broadcast = SporeBroadcast::new("S", vec!["D".into(), "L1".into()], 5);
        broadcast.plan(&mesh);
        assert!(!broadcast.paths.is_empty());
    }

    #[test]
    fn test_spore_broadcast_all_targets_reachable() {
        let mesh = rich_mesh();
        let mut broadcast = SporeBroadcast::new("S", vec!["D".into()], 10);
        broadcast.plan(&mesh);
        assert!(broadcast.paths.len() >= 1);
        for route in &broadcast.paths {
            assert_eq!(route.path.last().unwrap(), "D");
        }
    }

    #[test]
    fn test_multipath_delivery_2_of_3_succeeds() {
        // Build a mesh with 3 paths from S to D
        let mesh = Mesh::new()
            .with_nodes(vec![
                MeshNode::new("S", true, (0.0, 0.0)),
                MeshNode::new("M1", false, (1.0, 1.0)),
                MeshNode::new("M2", false, (1.0, 0.0)),
                MeshNode::new("M3", false, (1.0, -1.0)),
                MeshNode::new("D", true, (2.0, 0.0)),
            ])
            .with_links(vec![
                HyphaeLink::new("S", "M1", 100.0, 0.01),
                HyphaeLink::new("S", "M2", 100.0, 0.01),
                HyphaeLink::new("S", "M3", 100.0, 0.01),
                HyphaeLink::new("M1", "D", 100.0, 0.01),
                HyphaeLink::new("M2", "D", 100.0, 0.01),
                HyphaeLink::new("M3", "D", 100.0, 0.01),
            ]);

        let mut broadcast = SporeBroadcast::new("S", vec!["D".into()], 10)
            .with_delivery_threshold(2);
        broadcast.plan(&mesh);

        // Simulate one path failing (kill S→M2 link)
        let result = broadcast.simulate_delivery(&[("S", "M2")]);
        assert!(result.delivered); // 2 of 3 paths succeed
        assert_eq!(result.successful_paths, 2);
        assert_eq!(result.failed_paths, 1);
    }

    #[test]
    fn test_multipath_delivery_fails_when_too_many_fail() {
        let mesh = Mesh::new()
            .with_nodes(vec![
                MeshNode::new("S", true, (0.0, 0.0)),
                MeshNode::new("M1", false, (1.0, 1.0)),
                MeshNode::new("M2", false, (1.0, 0.0)),
                MeshNode::new("M3", false, (1.0, -1.0)),
                MeshNode::new("D", true, (2.0, 0.0)),
            ])
            .with_links(vec![
                HyphaeLink::new("S", "M1", 100.0, 0.01),
                HyphaeLink::new("S", "M2", 100.0, 0.01),
                HyphaeLink::new("S", "M3", 100.0, 0.01),
                HyphaeLink::new("M1", "D", 100.0, 0.01),
                HyphaeLink::new("M2", "D", 100.0, 0.01),
                HyphaeLink::new("M3", "D", 100.0, 0.01),
            ]);

        let mut broadcast = SporeBroadcast::new("S", vec!["D".into()], 10)
            .with_delivery_threshold(2);
        broadcast.plan(&mesh);

        // Kill 2 of 3 paths
        let result = broadcast.simulate_delivery(&[("S", "M1"), ("S", "M2")]);
        assert!(!result.delivered); // Only 1 path succeeds, need 2
    }

    #[test]
    fn test_per_target_delivery() {
        let mesh = rich_mesh();
        let mut broadcast = SporeBroadcast::new("S", vec!["D".into(), "L1".into()], 5)
            .with_delivery_threshold(1);
        broadcast.plan(&mesh);

        let results = broadcast.per_target_delivery(&[]);
        assert_eq!(results.len(), 2);
        for td in &results {
            assert!(td.delivered);
        }
    }

    // === FULL PIPELINE ===

    #[test]
    fn test_full_pipeline_mesh_route_relay_deliver() {
        // Build mesh → find route → relay with error correction → verify delivery
        let mesh = rich_mesh();
        let router = MeshRouter::new(&mesh);

        // Find best route
        let route = router.route("S", "D", &RoutingAlgorithm::MostReliable);
        assert!(route.is_valid());

        // Relay along that route
        let relay = HubRelay::new(0.6);
        let result = relay.relay_along_path(&route.path, &mesh, 0.0);

        // Should be delivered (reliability > 0.5)
        assert!(result.delivered);
        assert!(result.reliability > 0.5);
    }

    #[test]
    fn test_pipeline_with_initial_errors() {
        let mesh = rich_mesh();
        let router = MeshRouter::new(&mesh);
        let route = router.shortest_path("S", "D");

        let relay = HubRelay::new(0.5);
        let result = relay.relay_along_path(&route.path, &mesh, 0.1);
        // Cumulative correction should reduce initial error
        assert!(result.final_error_rate < 0.1);
    }

    #[test]
    fn test_pipeline_broadcast_resilience() {
        // Full pipeline: broadcast → resilience → verify
        let mesh = rich_mesh();
        let mut broadcast = SporeBroadcast::new("S", vec!["D".into()], 10)
            .with_delivery_threshold(2);
        broadcast.plan(&mesh);

        // Even with one link failure, broadcast should deliver
        let result = broadcast.simulate_delivery(&[("S", "H1")]);
        assert!(result.delivered);
    }

    // === SERDE ROUNDTRIPS ===

    #[test]
    fn test_serde_mesh_node() {
        let node = MeshNode::new("A", true, (1.5, 2.5));
        let json = serde_json::to_string(&node).unwrap();
        let back: MeshNode = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }

    #[test]
    fn test_serde_hyphae_link() {
        let link = HyphaeLink::new("A", "B", 100.0, 0.01);
        let json = serde_json::to_string(&link).unwrap();
        let back: HyphaeLink = serde_json::from_str(&json).unwrap();
        assert_eq!(link, back);
    }

    #[test]
    fn test_serde_mesh() {
        let mesh = triangle_mesh();
        let json = serde_json::to_string(&mesh).unwrap();
        let back: Mesh = serde_json::from_str(&json).unwrap();
        assert_eq!(mesh, back);
    }

    #[test]
    fn test_serde_route() {
        let route = Route::new(vec!["A".into(), "B".into(), "C".into()], 2.0, 0.97);
        let json = serde_json::to_string(&route).unwrap();
        let back: Route = serde_json::from_str(&json).unwrap();
        assert_eq!(route, back);
    }

    #[test]
    fn test_serde_resilience_report() {
        let report = ResilienceReport {
            k_resilience: 2,
            critical_links: vec!["A-B".into()],
            bottleneck_node: Some("C".into()),
            total_links: 5,
            total_nodes: 4,
            is_connected: true,
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: ResilienceReport = serde_json::from_str(&json).unwrap();
        assert_eq!(report, back);
    }

    #[test]
    fn test_serde_spore_broadcast() {
        let broadcast = SporeBroadcast::new("S", vec!["D".into()], 3);
        let json = serde_json::to_string(&broadcast).unwrap();
        let back: SporeBroadcast = serde_json::from_str(&json).unwrap();
        assert_eq!(broadcast, back);
    }

    #[test]
    fn test_serde_relay_result() {
        let result = RelayResult::new(
            vec!["A".into(), "B".into()],
            vec![RelayHop::new("B", 0.05, 0.5)],
            2.0,
        );
        let json = serde_json::to_string(&result).unwrap();
        let back: RelayResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }

    // === MISC EDGE CASES ===

    #[test]
    fn test_node_distance() {
        let a = MeshNode::new("A", false, (0.0, 0.0));
        let b = MeshNode::new("B", false, (3.0, 4.0));
        assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_connected_components() {
        let mesh = Mesh::new()
            .with_nodes(vec![
                MeshNode::new("A", false, (0.0, 0.0)),
                MeshNode::new("B", false, (1.0, 0.0)),
                MeshNode::new("C", false, (5.0, 5.0)),
            ])
            .with_links(vec![
                HyphaeLink::new("A", "B", 100.0, 0.01),
            ]);
        assert_eq!(mesh.connected_components(), 2);
    }

    #[test]
    fn test_default_hub_relay() {
        let relay = HubRelay::default();
        assert!((relay.correction_factor - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_route_none() {
        let route = Route::none();
        assert!(!route.is_valid());
        assert_eq!(route.hops, 0);
        assert_eq!(route.path.len(), 0);
    }

    #[test]
    fn test_find_all_paths() {
        let mesh = triangle_mesh();
        let router = MeshRouter::new(&mesh);
        let paths = router.find_all_paths("A", "B", 10);
        // In a triangle: A→B (direct), A→C→B
        assert!(paths.len() >= 1);
    }

    #[test]
    fn test_broadcast_result_fields() {
        let result = BroadcastResult {
            delivered: true,
            successful_paths: 3,
            failed_paths: 1,
            total_paths: 4,
        };
        assert_eq!(result.total_paths, 4);
        let json = serde_json::to_string(&result).unwrap();
        let back: BroadcastResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result, back);
    }
}
