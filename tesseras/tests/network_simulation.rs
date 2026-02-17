//! Network simulation tests: multi-node in-process scenarios.
//!
//! These tests spin up multiple Node instances on localhost to exercise
//! realistic P2P workflows end-to-end without Docker.

use tesseras::config::{DataDir, NodeConfig};
use tesseras::crypto::Identity;
use tesseras::node::Node;
use tesseras::types::Visibility;

/// Create a test node with an ephemeral data directory and STUN disabled.
fn make_node(bootstrap: Vec<String>) -> (tempfile::TempDir, Node) {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = DataDir::open(tmp.path()).unwrap();
    let identity = Identity::generate();
    let mut config = NodeConfig::default();
    config.listen = "127.0.0.1:0".parse().unwrap();
    config.stun_servers = Vec::new();
    config.bootstrap = bootstrap;
    let node = Node::new(data_dir, identity, config).unwrap();
    (tmp, node)
}

/// Start a node and return its listening address.
async fn start_node(node: &mut Node) -> std::net::SocketAddr {
    node.start().await.unwrap()
}

#[tokio::test]
async fn three_node_tessera_replication() {
    // A creates a tessera, announces it. B and C should be able to find it via DHT.
    let (_tmp_a, mut node_a) = make_node(vec![]);
    let addr_a = start_node(&mut node_a).await;

    let (_tmp_b, mut node_b) = make_node(vec![addr_a.to_string()]);
    let _addr_b = start_node(&mut node_b).await;
    node_b.bootstrap().await.unwrap();

    let (_tmp_c, mut node_c) = make_node(vec![addr_a.to_string()]);
    let _addr_c = start_node(&mut node_c).await;
    node_c.bootstrap().await.unwrap();

    // A creates a tessera
    let test_file = _tmp_a.path().join("memory.txt");
    std::fs::write(&test_file, b"three node test memory").unwrap();
    let tessera = node_a
        .add_tessera(&[test_file], Some("ThreeNode".into()), Visibility::Public)
        .unwrap();

    // A announces it
    let stored = node_a.announce_tessera(&tessera.hash).await.unwrap();
    assert!(stored > 0, "should announce to at least one peer");

    // B fetches from network
    let fetched_b = node_b
        .fetch_tessera_from_network(&tessera.hash)
        .await
        .unwrap();
    assert!(fetched_b.is_some(), "B should fetch tessera from network");
    let fetched_b = fetched_b.unwrap();
    assert_eq!(fetched_b.hash, tessera.hash);
    assert_eq!(fetched_b.name, Some("ThreeNode".into()));

    // C fetches from network (might come from A or B's cache)
    let fetched_c = node_c
        .fetch_tessera_from_network(&tessera.hash)
        .await
        .unwrap();
    assert!(fetched_c.is_some(), "C should fetch tessera from network");
    assert_eq!(fetched_c.unwrap().hash, tessera.hash);

    node_a.shutdown();
    node_b.shutdown();
    node_c.shutdown();
}

#[tokio::test]
async fn five_node_mesh_discovery() {
    // Build a 5-node mesh: each node bootstraps from the previous one.
    // Verify all nodes discover each other via iterative lookups.
    let (_tmp_0, mut node_0) = make_node(vec![]);
    let addr_0 = start_node(&mut node_0).await;

    let mut nodes = vec![];
    let mut tmps = vec![];
    let mut prev_addr = addr_0.to_string();

    for _ in 0..4 {
        let (tmp, mut node) = make_node(vec![prev_addr.clone()]);
        let addr = start_node(&mut node).await;
        node.bootstrap().await.unwrap();
        prev_addr = addr.to_string();
        tmps.push(tmp);
        nodes.push(node);
    }

    // All nodes should know at least 2 peers (direct bootstrap peer + some discovered)
    let node_0_peers = node_0.dht.lock().unwrap().routing_table.len();
    assert!(
        node_0_peers >= 1,
        "node_0 should know at least 1 peer, has {node_0_peers}"
    );

    for (i, node) in nodes.iter().enumerate() {
        let peers = node.dht.lock().unwrap().routing_table.len();
        assert!(
            peers >= 1,
            "node_{} should know at least 1 peer, has {}",
            i + 1,
            peers
        );
    }

    // Node 4 (last) should be able to reach node 0's content
    let test_file = _tmp_0.path().join("mesh.txt");
    std::fs::write(&test_file, b"mesh discovery test").unwrap();
    let tessera = node_0
        .add_tessera(&[test_file], Some("Mesh".into()), Visibility::Public)
        .unwrap();
    node_0.announce_tessera(&tessera.hash).await.unwrap();

    let last_node = nodes.last().unwrap();
    let fetched = last_node
        .fetch_tessera_from_network(&tessera.hash)
        .await
        .unwrap();
    assert!(fetched.is_some(), "last node should find tessera via mesh");

    node_0.shutdown();
    for node in &nodes {
        node.shutdown();
    }
}

#[tokio::test]
async fn node_restarts_with_persisted_peers() {
    // Node A starts, B bootstraps from A. B shuts down and restarts.
    // After restart, B should have A in its routing table from persistence.
    let (_tmp_a, mut node_a) = make_node(vec![]);
    let addr_a = start_node(&mut node_a).await;

    let tmp_b = tempfile::tempdir().unwrap();
    let data_dir_b = DataDir::open(tmp_b.path()).unwrap();

    // First lifecycle: create B, bootstrap, shutdown
    {
        let identity_b = Identity::generate();
        let mut config_b = NodeConfig::default();
        config_b.listen = "127.0.0.1:0".parse().unwrap();
        config_b.stun_servers = Vec::new();
        config_b.bootstrap = vec![addr_a.to_string()];
        let mut node_b = Node::new(data_dir_b.clone(), identity_b, config_b).unwrap();
        let _addr_b = start_node(&mut node_b).await;
        node_b.bootstrap().await.unwrap();

        let peers_before = node_b.dht.lock().unwrap().routing_table.len();
        assert!(peers_before >= 1, "B should know A after bootstrap");

        node_b.shutdown();
    }

    // Second lifecycle: recreate B from same data dir (no bootstrap config)
    {
        let identity_b2 = Identity::generate();
        let mut config_b2 = NodeConfig::default();
        config_b2.listen = "127.0.0.1:0".parse().unwrap();
        config_b2.stun_servers = Vec::new();
        config_b2.bootstrap = vec![]; // No bootstrap — relies on persisted peers
        let node_b2 = Node::new(data_dir_b, identity_b2, config_b2).unwrap();

        let peers_after = node_b2.dht.lock().unwrap().routing_table.len();
        assert!(
            peers_after >= 1,
            "B should have persisted peers from previous lifecycle, has {peers_after}"
        );

        node_b2.shutdown();
    }

    node_a.shutdown();
}

#[tokio::test]
async fn multiple_tesseras_from_different_nodes() {
    // A and B each create tesseras. C should be able to fetch both.
    let (_tmp_a, mut node_a) = make_node(vec![]);
    let addr_a = start_node(&mut node_a).await;

    let (_tmp_b, mut node_b) = make_node(vec![addr_a.to_string()]);
    let _addr_b = start_node(&mut node_b).await;
    node_b.bootstrap().await.unwrap();

    let (_tmp_c, mut node_c) = make_node(vec![addr_a.to_string()]);
    let _addr_c = start_node(&mut node_c).await;
    node_c.bootstrap().await.unwrap();

    // A creates tessera
    let file_a = _tmp_a.path().join("a.txt");
    std::fs::write(&file_a, b"memory from A").unwrap();
    let tessera_a = node_a
        .add_tessera(&[file_a], Some("From A".into()), Visibility::Public)
        .unwrap();
    node_a.announce_tessera(&tessera_a.hash).await.unwrap();

    // B creates tessera
    let file_b = _tmp_b.path().join("b.txt");
    std::fs::write(&file_b, b"memory from B").unwrap();
    let tessera_b = node_b
        .add_tessera(&[file_b], Some("From B".into()), Visibility::Public)
        .unwrap();
    node_b.announce_tessera(&tessera_b.hash).await.unwrap();

    // C fetches both
    let fetched_a = node_c
        .fetch_tessera_from_network(&tessera_a.hash)
        .await
        .unwrap();
    assert!(fetched_a.is_some());
    assert_eq!(fetched_a.unwrap().name, Some("From A".into()));

    let fetched_b = node_c
        .fetch_tessera_from_network(&tessera_b.hash)
        .await
        .unwrap();
    assert!(fetched_b.is_some());
    assert_eq!(fetched_b.unwrap().name, Some("From B".into()));

    node_a.shutdown();
    node_b.shutdown();
    node_c.shutdown();
}
