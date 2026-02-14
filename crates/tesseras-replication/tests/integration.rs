//! Integration test: full replication cycle.

use std::sync::{Arc, Mutex};

use mockall::mock;
use tesseras_core::ports::{BlobStore, DhtPort};
use tesseras_core::replication::*;
use tesseras_core::types::NodeId;
use tesseras_core::*;
use tesseras_replication::*;
use tesseras_storage::{FsFragmentStore, SqliteReciprocityLedger};

fn hash(fill: u8) -> ContentHash {
    ContentHash::new([fill; 32])
}
fn node(fill: u8) -> NodeId {
    NodeId::new([fill; 20])
}

mock! {
    pub Dht {}
    #[async_trait::async_trait]
    impl DhtPort for Dht {
        async fn find_closest_nodes(&self, target: &NodeId) -> Vec<NodeInfo>;
        async fn replicate_fragment(
            &self,
            target: &NodeInfo,
            fragment: &FragmentEnvelope,
        ) -> Result<ReplicateAck, CoreError>;
        async fn request_attestation(
            &self,
            target: &NodeInfo,
            tessera_hash: &ContentHash,
        ) -> Result<Attestation, CoreError>;
        async fn ping(&self, target: &NodeInfo) -> bool;
    }
}

mock! {
    pub Blobs {}
    impl BlobStore for Blobs {
        fn write(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
            data: &[u8],
        ) -> Result<(), CoreError>;
        fn read(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
        ) -> Result<Vec<u8>, CoreError>;
        fn exists(
            &self,
            tessera_hash: &ContentHash,
            memory_hash: &ContentHash,
            name: &str,
        ) -> Result<bool, CoreError>;
        fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError>;
    }
}

fn make_node_info(fill: u8, port: u16) -> NodeInfo {
    NodeInfo {
        identity: NodeIdentity {
            node_id: node(fill),
            public_key: [fill; 32],
            nonce: 0,
        },
        addr: std::net::SocketAddr::from(([10, 0, fill, 1], port)),
        alt_addrs: vec![],
        capabilities: Capabilities::phase2_default(),
    }
}

/// Helper: create a ReplicationService backed by in-memory SQLite + tempdir fragments.
fn create_service_with_real_storage(
    node_fill: u8,
    dht: MockDht,
    blobs: MockBlobs,
    dir: &std::path::Path,
) -> ReplicationService {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    tesseras_storage::run_migrations(&conn).unwrap();
    let conn = Arc::new(Mutex::new(conn));

    let fragment_store = FsFragmentStore::new(Arc::clone(&conn), dir.join("fragments"));
    let ledger = SqliteReciprocityLedger::new(conn);

    ReplicationService::new(
        NodeIdentity {
            node_id: node(node_fill),
            public_key: [node_fill; 32],
            nonce: 0,
        },
        Box::new(dht),
        Box::new(fragment_store),
        Box::new(ledger),
        Box::new(blobs),
        ReplicationConfig::default(),
    )
}

#[tokio::test]
async fn replicate_and_receive_medium_tessera() {
    let dir = tempfile::TempDir::new().unwrap();

    // Setup owner's DHT mock: finds 7 peers, all accept
    let mut owner_dht = MockDht::new();
    let peers: Vec<NodeInfo> = (1..=7).map(|i| make_node_info(i, 4433)).collect();
    let peers_clone = peers.clone();
    owner_dht
        .expect_find_closest_nodes()
        .returning(move |_| peers_clone.clone());

    // Collect received envelopes to forward to receiver
    let received_envelopes: Arc<Mutex<Vec<FragmentEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let envelopes_clone = Arc::clone(&received_envelopes);
    owner_dht
        .expect_replicate_fragment()
        .returning(move |_, frag| {
            envelopes_clone.lock().unwrap().push(frag.clone());
            Ok(ReplicateAck {
                accepted: true,
                fragments_held: vec![frag.id.index],
            })
        });

    let owner = create_service_with_real_storage(0xff, owner_dht, MockBlobs::new(), dir.path());

    // 1. Create a 10 MB tessera (medium tier)
    let data = vec![0xdd; 10 * 1024 * 1024];
    let tessera_hash = hash(0x01);

    // 2. Replicate → should produce 24 fragments
    let report = owner.replicate_tessera(&tessera_hash, &data).await.unwrap();
    assert_eq!(report.fragments_distributed, 24); // 16 data + 8 parity

    // 3. Create receiver service and receive all fragments
    let recv_dir = tempfile::TempDir::new().unwrap();
    let recv_dht = MockDht::new();
    let receiver =
        create_service_with_real_storage(0x01, recv_dht, MockBlobs::new(), recv_dir.path());

    // Clone envelopes so we can drop the MutexGuard before awaiting.
    let envelopes_vec: Vec<_> = received_envelopes.lock().unwrap().clone();
    assert_eq!(envelopes_vec.len(), 24);

    for envelope in &envelopes_vec {
        let ack = receiver
            .receive_fragment(envelope.clone(), &node(0xff))
            .await
            .unwrap();
        assert!(ack.accepted);
    }

    // 5. Verify all fragments stored correctly
    let status = receiver.status(&tessera_hash).unwrap();
    assert_eq!(status.fragments_held, 24);

    // 6. Check reciprocity: receiver stored for owner
    let balance = receiver.ledger().balance(&node(0xff)).unwrap();
    assert!(balance < 0); // receiver stored for owner → negative balance
}

#[tokio::test]
async fn replicate_small_tessera_whole_file() {
    let dir = tempfile::TempDir::new().unwrap();

    let mut dht = MockDht::new();
    let peers: Vec<NodeInfo> = (1..=7).map(|i| make_node_info(i, 4433)).collect();
    let peers_clone = peers.clone();
    dht.expect_find_closest_nodes()
        .returning(move |_| peers_clone.clone());
    dht.expect_replicate_fragment().times(7).returning(|_, _| {
        Ok(ReplicateAck {
            accepted: true,
            fragments_held: vec![],
        })
    });

    let service = create_service_with_real_storage(0xff, dht, MockBlobs::new(), dir.path());

    // 1 KB tessera — small tier
    let data = vec![0xaa; 1000];
    let tessera_hash = hash(0x02);

    let report = service
        .replicate_tessera(&tessera_hash, &data)
        .await
        .unwrap();
    assert_eq!(report.peers_accepted, 7);
    assert_eq!(report.fragments_distributed, 7);
    // No erasure coding — fragments should be empty in owner's local store
    // (small tier doesn't store fragments locally, it pushes raw data)
}

#[tokio::test]
async fn receive_rejects_tampered_fragment() {
    let dir = tempfile::TempDir::new().unwrap();
    let dht = MockDht::new();
    let service = create_service_with_real_storage(0x01, dht, MockBlobs::new(), dir.path());

    // Create a valid envelope
    let data = vec![0xaa; 64];
    let checksum = ContentHash::new(blake3::hash(&data).into());
    let plan = FragmentPlan::new(hash(0x01), 100_000_000).unwrap();
    let id = FragmentId::new(hash(0x01), 0, 16, checksum);
    let mut envelope = FragmentEnvelope {
        id,
        plan,
        original_tessera_size: 100_000_000,
        fragment_size: 64,
        data,
    };

    // Tamper with the data
    envelope.data = vec![0xbb; 64];

    let result = service.receive_fragment(envelope, &node(0xff)).await;
    assert!(matches!(
        result,
        Err(ReplicationError::ChecksumMismatch { .. })
    ));
}
