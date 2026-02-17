//! Integration test: RPC client ↔ daemon listener round-trip.

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
use tesseras_rpc::{DaemonClient, Request};
use tokio::sync::watch;

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

async fn make_dht_engine() -> Arc<DhtEngine> {
    let net = SimNetwork::new();
    let addr: SocketAddr = "127.0.0.1:19999".parse().unwrap();
    let transport = net.create_transport(addr, 256).await;
    let pubkey = [0xFFu8; 32];
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
    let blob_store: Arc<dyn BlobStore> = Arc::new(MockBlobs::new());

    let dht = MockDht::new();
    let ledger = tesseras_storage::SqliteReciprocityLedger::new(Arc::clone(&conn));

    let node_id = NodeId::new([0x01; 20]);
    let identity = NodeIdentity {
        node_id,
        public_key: [0x01; 32],
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
    })
}

#[tokio::test]
async fn publish_unknown_hash_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let socket_path = tmp.path().join("test.sock");

    let handler = make_handler(tmp.path(), make_dht_engine().await);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let sp = socket_path.clone();
    let h = Arc::clone(&handler);
    let listener = tokio::spawn(async move {
        tesseras_daemon::rpc::run_listener(sp, h, shutdown_rx).await;
    });

    // Give listener time to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect and publish a hash that doesn't exist in storage
    let sp = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::Publish {
            hash: ContentHash::new([0xaa; 32]),
        })
    })
    .await
    .unwrap();

    // Should get an error because the tessera doesn't exist
    assert!(result.is_err(), "expected error for unknown hash, got: {result:?}");

    shutdown_tx.send(true).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), listener).await;
}

#[tokio::test]
async fn status_unknown_hash_returns_error() {
    let tmp = tempfile::TempDir::new().unwrap();
    let socket_path = tmp.path().join("test.sock");

    let handler = make_handler(tmp.path(), make_dht_engine().await);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let sp = socket_path.clone();
    let h = Arc::clone(&handler);
    let listener = tokio::spawn(async move {
        tesseras_daemon::rpc::run_listener(sp, h, shutdown_rx).await;
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let sp = socket_path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut client = DaemonClient::connect(&sp).unwrap();
        client.call(&Request::TesseraStatus {
            hash: ContentHash::new([0xbb; 32]),
        })
    })
    .await
    .unwrap();

    assert!(result.is_err(), "expected error for unknown hash, got: {result:?}");

    shutdown_tx.send(true).unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(2), listener).await;
}
