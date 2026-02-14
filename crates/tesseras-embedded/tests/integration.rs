//! Full lifecycle integration test for tesseras-embedded.

use tesseras_embedded::{CreateMemoryRequest, EmbeddedNode};

#[test]
fn full_lifecycle() {
    let dir = tempfile::TempDir::new().unwrap();
    let data_dir = dir.path().to_str().unwrap().to_string();

    // 1. Start node
    let node = EmbeddedNode::start(data_dir.clone()).expect("start");

    // 2. Create identity
    let identity = node
        .create_identity("Integration Test User".to_string(), None)
        .expect("create identity");
    assert_eq!(identity.name, "Integration Test User");

    // 3. Create a memory
    let media_path = dir.path().join("test.jpg");
    std::fs::write(&media_path, b"fake jpeg content for testing").unwrap();

    let memory = node
        .create_memory(CreateMemoryRequest {
            media_path: media_path.to_str().unwrap().to_string(),
            context_text: Some("Integration test memory".to_string()),
            memory_type: tesseras_core::MemoryType::Moment,
            visibility: tesseras_core::Visibility::Private,
            location_description: None,
            location_lat: None,
            location_lon: None,
            tags: vec!["test".to_string()],
            people: vec![],
        })
        .expect("create memory");

    // 4. Get timeline
    let timeline = node.get_timeline(0, 10).expect("get timeline");
    assert_eq!(timeline.len(), 1);
    assert_eq!(timeline[0].hash, memory.hash);

    // 5. Get network stats
    let stats = node.get_network_stats().expect("get stats");
    assert!(stats.uptime_secs < 30);

    // 6. Stop
    node.stop().expect("stop");

    // 7. Restart and verify persistence
    let node = EmbeddedNode::start(data_dir).expect("restart");
    let identity = node.get_identity().expect("get identity").expect("exists");
    assert_eq!(identity.name, "Integration Test User");

    let timeline = node.get_timeline(0, 10).expect("timeline after restart");
    assert_eq!(timeline.len(), 1);

    node.stop().expect("final stop");
}

#[test]
fn restart_cycle_no_corruption() {
    let dir = tempfile::TempDir::new().unwrap();
    let data_dir = dir.path().to_str().unwrap().to_string();

    // Simulate paused/resumed lifecycle: start -> stop x 3
    for i in 0..3 {
        let node = EmbeddedNode::start(data_dir.clone())
            .unwrap_or_else(|e| panic!("start cycle {i} failed: {e}"));
        node.stop()
            .unwrap_or_else(|e| panic!("stop cycle {i} failed: {e}"));
    }

    // After 3 restart cycles, node should still work and data intact
    let node = EmbeddedNode::start(data_dir).expect("final start");
    let stats = node.get_network_stats().expect("stats after restarts");
    assert!(stats.uptime_secs < 5);
    node.stop().expect("final stop");
}
