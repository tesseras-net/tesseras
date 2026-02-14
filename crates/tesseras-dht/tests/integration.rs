use std::net::SocketAddr;
use std::sync::Arc;

use tesseras_core::*;
use tesseras_dht::{config::DhtConfig, engine::DhtEngine, pow};
use tesseras_net::SimNetwork;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn test_config() -> DhtConfig {
    DhtConfig {
        rpc_timeout: std::time::Duration::from_millis(500),
        ..DhtConfig::default()
    }
}

async fn create_engine(net: &SimNetwork, port: u16) -> Arc<DhtEngine> {
    let transport = net.create_transport(addr(port), 1024).await;
    let mut pubkey = [0u8; 32];
    pubkey[0] = (port >> 8) as u8;
    pubkey[1] = port as u8;
    let identity = pow::generate_node_identity(&pubkey);
    DhtEngine::new(identity, Box::new(transport), test_config())
}

/// Spawn an engine's run loop. Returns shutdown sender and join handle.
fn spawn_engine(
    engine: &Arc<DhtEngine>,
) -> (
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<()>,
) {
    let (tx, rx) = tokio::sync::watch::channel(false);
    let e = Arc::clone(engine);
    let handle = tokio::spawn(async move {
        e.run(rx).await;
    });
    (tx, handle)
}

#[tokio::test]
async fn three_node_bootstrap() {
    let net = SimNetwork::new();
    let e1 = create_engine(&net, 10001).await;
    let e2 = create_engine(&net, 10002).await;
    let e3 = create_engine(&net, 10003).await;

    // Start all run loops so responses get routed via pending-requests map
    let (s1, _) = spawn_engine(&e1);
    let (s2, _) = spawn_engine(&e2);
    let (s3, _) = spawn_engine(&e3);
    tokio::task::yield_now().await;

    // e2 bootstraps from e1
    e2.bootstrap(&[addr(10001)]).await.unwrap();
    assert!(
        e2.routing_table_size().await >= 1,
        "e2 should know at least e1"
    );

    e3.bootstrap(&[addr(10002)]).await.unwrap();
    assert!(
        e3.routing_table_size().await >= 1,
        "e3 should know at least e2"
    );

    // e1 should have learned about e2 from the Ping during bootstrap
    assert!(
        e1.routing_table_size().await >= 1,
        "e1 should know about e2"
    );

    s1.send(true).ok();
    s2.send(true).ok();
    s3.send(true).ok();
}

#[tokio::test]
async fn ten_node_lookup_convergence() {
    let net = SimNetwork::new();
    let mut engines: Vec<Arc<DhtEngine>> = Vec::new();
    let mut shutdowns = Vec::new();

    // Create 10 engines
    for i in 0u16..10 {
        engines.push(create_engine(&net, 20001 + i).await);
    }

    // Start all run loops so responses get routed via pending-requests map
    for engine in &engines {
        let (s, _) = spawn_engine(engine);
        shutdowns.push(s);
    }
    tokio::task::yield_now().await;

    // Bootstrap each subsequent engine from the previous one
    for (i, engine) in engines.iter().enumerate().skip(1) {
        let prev_port = 20001 + (i as u16 - 1);
        engine.bootstrap(&[addr(prev_port)]).await.unwrap();
        tokio::task::yield_now().await;
    }

    // Every node should know at least 1 peer
    for (i, e) in engines.iter().enumerate() {
        let size = e.routing_table_size().await;
        assert!(
            size >= 1,
            "node {i} should know at least 1 peer, has {size}"
        );
    }

    // Last node should have discovered peers beyond its direct bootstrap seed
    // through iterative lookup (find_closest_nodes queries returned peers)
    let last_size = engines[9].routing_table_size().await;
    assert!(
        last_size >= 2,
        "last node should know at least 2 peers via iterative lookup, has {last_size}"
    );

    for s in &shutdowns {
        s.send(true).ok();
    }
}

#[tokio::test]
async fn publish_and_find_tessera() {
    let net = SimNetwork::new();
    let e1 = create_engine(&net, 30001).await;
    let e2 = create_engine(&net, 30002).await;
    let e3 = create_engine(&net, 30003).await;

    // Start all run loops so responses get routed via pending-requests map
    let (s1, _) = spawn_engine(&e1);
    let (s2, _) = spawn_engine(&e2);
    let (s3, _) = spawn_engine(&e3);
    tokio::task::yield_now().await;

    // e2 bootstraps and publishes a pointer
    e2.bootstrap(&[addr(30001)]).await.unwrap();

    let pointer = TesseraPointer {
        tessera_hash: ContentHash::new([0xaa; 32]),
        size_bytes: 1024,
        holders: vec![],
        visibility: Visibility::Public,
        created_at: chrono::Utc::now(),
    };
    let acks = e2.publish(pointer.clone()).await.unwrap();
    assert!(acks > 0, "should receive at least one store ack");

    // e1 should now hold the pointer
    assert!(
        e1.store_size().await > 0,
        "storage node should hold the pointer"
    );

    // e3 bootstraps and finds the pointer
    e3.bootstrap(&[addr(30001)]).await.unwrap();
    let found = e3.find_tessera(&pointer.tessera_hash).await.unwrap();
    assert!(found.is_some(), "should find the published pointer");
    let found = found.unwrap();
    assert_eq!(found.tessera_hash, pointer.tessera_hash);
    assert_eq!(found.size_bytes, 1024);

    s1.send(true).ok();
    s2.send(true).ok();
    s3.send(true).ok();
}

#[tokio::test]
async fn node_departure_detected() {
    let net = SimNetwork::new();
    let e1 = create_engine(&net, 40001).await;
    let e2 = create_engine(&net, 40002).await;

    // Start both run loops so responses get routed
    let (s1, h1) = spawn_engine(&e1);
    let (s2, h2) = spawn_engine(&e2);
    tokio::task::yield_now().await;

    // Bootstrap e2 from e1
    e2.bootstrap(&[addr(40001)]).await.unwrap();
    assert!(e2.routing_table_size().await >= 1);

    // Shutdown e1
    s1.send(true).unwrap();
    h1.await.unwrap();

    // Ping should fail (e1 is shut down, RPC times out)
    assert!(
        !e2.ping(addr(40001)).await,
        "ping should fail after node departure"
    );

    s2.send(true).ok();
    let _ = h2.await;
}

#[tokio::test]
async fn pow_rejection() {
    let net = SimNetwork::new();
    let e_good = create_engine(&net, 50001).await;
    let (s, _) = spawn_engine(&e_good);

    // Create engine with invalid PoW
    let transport = net.create_transport(addr(50002), 1024).await;
    let bad_identity = NodeIdentity {
        node_id: NodeId::new([0xff; 20]),
        public_key: [0x01; 32],
        nonce: 0,
    };
    let e_bad = DhtEngine::new(bad_identity, Box::new(transport), test_config());

    // Start e_bad's run loop so it can receive the Pong response
    let (s_bad, _) = spawn_engine(&e_bad);
    tokio::task::yield_now().await;

    // Bad node pings good node — gets a Pong (good node always responds)
    // but good node does NOT add the bad node to its routing table
    assert!(e_bad.ping(addr(50001)).await, "should get Pong response");
    assert_eq!(
        e_good.routing_table_size().await,
        0,
        "good node should not add node with invalid PoW"
    );

    s.send(true).ok();
    s_bad.send(true).ok();
}
