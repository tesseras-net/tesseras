//! End-to-end integration tests: exercise new RPC operations through the full
//! client → listener → handler → storage round-trip.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mockall::mock;
use tesseras_core::ports::{BlobStore, DhtPort};
use tesseras_core::replication::*;
use tesseras_core::types::NodeId;
use tesseras_core::*;
use tesseras_dht::config::DhtConfig;
use tesseras_dht::engine::DhtEngine;
use tesseras_dht::pow;
use tesseras_net::SimNetwork;
use tesseras_rpc::{DaemonClient, Request, Response};
use tokio::sync::watch;

// ── Mocks ──────────────────────────────────────────────────────────

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

// ── Helpers ────────────────────────────────────────────────────────

async fn make_dht_engine() -> Arc<DhtEngine> {
    let net = SimNetwork::new();
    let addr: SocketAddr = "127.0.0.1:19998".parse().unwrap();
    let transport = net.create_transport(addr, 256).await;
    let pubkey = [0xEEu8; 32];
    let identity = pow::generate_node_identity(&pubkey);
    DhtEngine::new(identity, Box::new(transport), DhtConfig::default())
}

fn make_handler(
    dir: &std::path::Path,
    dht_engine: Arc<DhtEngine>,
) -> Arc<tesseras_daemon::rpc::handler::RpcHandler> {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    tesseras_storage::run_migrations(&conn).unwrap();
    let conn = Arc::new(Mutex::new(conn));

    let cas = Arc::new(tesseras_storage::CasStore::new(
        Arc::clone(&conn),
        dir.join("cas"),
    ));

    let fragment_store = Arc::new(tesseras_storage::FsFragmentStore::new(
        Arc::clone(&conn),
        Arc::clone(&cas),
    ));

    let mut blobs = MockBlobs::new();
    blobs.expect_delete_tessera().returning(|_| Ok(()));
    let blob_store: Arc<dyn BlobStore> = Arc::new(blobs);

    let dht = MockDht::new();
    let ledger = tesseras_storage::SqliteReciprocityLedger::new(Arc::clone(&conn));

    let node_id = NodeId::new([0x02; 20]);
    let identity = NodeIdentity {
        node_id,
        public_key: [0x02; 32],
        nonce: 0,
    };

    let replication = tesseras_replication::ReplicationService::new(
        identity,
        Box::new(dht),
        Box::new(tesseras_storage::FsFragmentStore::new(
            Arc::clone(&conn),
            Arc::clone(&cas),
        )),
        Box::new(ledger),
        Box::new(MockBlobs::new()),
        tesseras_replication::ReplicationConfig::default(),
    );

    Arc::new(tesseras_daemon::rpc::handler::RpcHandler {
        tessera_repo: Arc::new(tesseras_storage::SqliteTesseraRepository::new(
            Arc::clone(&conn),
        )),
        memory_repo: Arc::new(tesseras_storage::SqliteMemoryRepository::new(
            Arc::clone(&conn),
        )),
        blob_store,
        fragment_store,
        replication: Arc::new(replication),
        cas,
        dht_engine,
        tombstone_repo: Arc::new(tesseras_storage::SqliteTombstoneRepository::new(
            Arc::clone(&conn),
        )),
        circle_repo: Arc::new(tesseras_storage::SqliteCircleRepository::new(
            Arc::clone(&conn),
        )),
        operation_queue: Arc::new(tesseras_storage::SqliteOperationQueue::new(
            Arc::clone(&conn),
        )),
        data_dir: dir.to_path_buf(),
        start_time: std::time::Instant::now(),
    })
}

/// Spin up the listener, run a closure with a connected client, then shut down.
async fn with_daemon<F, T>(f: F) -> T
where
    F: FnOnce(std::path::PathBuf) -> T + Send + 'static,
    T: Send + 'static,
{
    let tmp = tempfile::TempDir::new().unwrap();
    let socket_path = tmp.path().join("e2e.sock");

    let handler = make_handler(tmp.path(), make_dht_engine().await);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let sp = socket_path.clone();
    let h = Arc::clone(&handler);
    let listener = tokio::spawn(async move {
        tesseras_daemon::rpc::run_listener(sp, h, shutdown_rx).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let sp = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || f(sp)).await.unwrap();

    shutdown_tx.send(true).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), listener).await;
    result
}

// ── Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn list_returns_empty_initially() {
    let resp = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::List { circle: None }).unwrap()
    })
    .await;

    match resp {
        Response::Listed { records } => {
            assert!(records.is_empty(), "expected empty list, got {} records", records.len());
        }
        other => panic!("expected Listed response, got: {other:?}"),
    }
}

#[tokio::test]
async fn show_unknown_hash_returns_error() {
    let err = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::Show {
            hash: "deadbeef".to_string(),
        })
    })
    .await;

    match err {
        Err(tesseras_rpc::RpcError::DaemonError { code, .. }) => {
            assert!(
                matches!(code, tesseras_rpc::ErrorCode::NotFound),
                "expected NotFound, got: {code:?}"
            );
        }
        other => panic!("expected DaemonError NotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn delete_unknown_hash_returns_error() {
    let err = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::Delete {
            hash: "cafebabe".to_string(),
        })
    })
    .await;

    match err {
        Err(tesseras_rpc::RpcError::DaemonError { code, .. }) => {
            assert!(
                matches!(code, tesseras_rpc::ErrorCode::NotFound),
                "expected NotFound, got: {code:?}"
            );
        }
        other => panic!("expected DaemonError NotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn circle_create_list_delete_roundtrip() {
    let results = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();

        // Create
        let create_resp = client
            .call(&Request::CircleCreate {
                name: "family".to_string(),
            })
            .unwrap();

        // List
        let list_resp = client.call(&Request::CircleList { name: None }).unwrap();

        // Delete
        let delete_resp = client
            .call(&Request::CircleDelete {
                name: "family".to_string(),
            })
            .unwrap();

        // List again
        let list_after = client.call(&Request::CircleList { name: None }).unwrap();

        (create_resp, list_resp, delete_resp, list_after)
    })
    .await;

    match results.0 {
        Response::CircleCreated { name } => assert_eq!(name, "family"),
        other => panic!("expected CircleCreated, got: {other:?}"),
    }

    match results.1 {
        Response::Circles { circles } => {
            assert_eq!(circles.len(), 1);
            assert_eq!(circles[0].name, "family");
        }
        other => panic!("expected Circles, got: {other:?}"),
    }

    match results.2 {
        Response::CircleDeleted { name } => assert_eq!(name, "family"),
        other => panic!("expected CircleDeleted, got: {other:?}"),
    }

    match results.3 {
        Response::Circles { circles } => {
            assert!(circles.is_empty(), "expected empty after delete");
        }
        other => panic!("expected Circles, got: {other:?}"),
    }
}

#[tokio::test]
async fn circle_add_remove_member() {
    let results = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();

        // Create circle first
        client
            .call(&Request::CircleCreate {
                name: "friends".to_string(),
            })
            .unwrap();

        // Add member
        let add_resp = client
            .call(&Request::CircleAddMember {
                circle: "friends".to_string(),
                alias: "alice".to_string(),
                pubkey: "aa".repeat(32),
            })
            .unwrap();

        // Remove member
        let remove_resp = client
            .call(&Request::CircleRemoveMember {
                circle: "friends".to_string(),
                alias: "alice".to_string(),
            })
            .unwrap();

        (add_resp, remove_resp)
    })
    .await;

    assert!(
        matches!(results.0, Response::CircleMemberAdded),
        "expected CircleMemberAdded, got: {:?}",
        results.0
    );
    assert!(
        matches!(results.1, Response::CircleMemberRemoved),
        "expected CircleMemberRemoved, got: {:?}",
        results.1
    );
}

#[tokio::test]
async fn node_status_returns_info() {
    let resp = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::Status).unwrap()
    })
    .await;

    match resp {
        Response::NodeStatus {
            peer_count,
            node_id,
            uptime_secs,
            ..
        } => {
            assert_eq!(peer_count, 0, "no peers in test");
            assert!(!node_id.is_empty(), "node_id should not be empty");
            // Uptime should be very small since we just started
            assert!(uptime_secs < 60, "uptime too large: {uptime_secs}");
        }
        other => panic!("expected NodeStatus, got: {other:?}"),
    }
}

#[tokio::test]
async fn queue_status_returns_empty() {
    let resp = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::QueueStatus).unwrap()
    })
    .await;

    match resp {
        Response::QueueEntries { entries } => {
            assert!(entries.is_empty(), "expected empty queue");
        }
        other => panic!("expected QueueEntries, got: {other:?}"),
    }
}

#[tokio::test]
async fn duplicate_circle_create_returns_error() {
    let err = with_daemon(|sp| {
        let mut client = DaemonClient::connect(&sp).unwrap();

        // Create once
        client
            .call(&Request::CircleCreate {
                name: "dup".to_string(),
            })
            .unwrap();

        // Create again — should fail
        client.call(&Request::CircleCreate {
            name: "dup".to_string(),
        })
    })
    .await;

    match err {
        Err(tesseras_rpc::RpcError::DaemonError { code, .. }) => {
            assert!(
                matches!(code, tesseras_rpc::ErrorCode::AlreadyExists),
                "expected AlreadyExists, got: {code:?}"
            );
        }
        other => panic!("expected DaemonError AlreadyExists, got: {other:?}"),
    }
}
