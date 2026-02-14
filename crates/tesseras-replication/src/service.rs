use tesseras_core::ports::{BlobStore, DhtPort, FragmentStore, ReciprocityLedger};
use tesseras_core::replication::{
    Attestation, AttestationEntry, FragmentEnvelope, ReplicateAck, MAX_TESSERA_SIZE,
};
use tesseras_core::types::NodeId;
use tesseras_core::{ContentHash, NodeIdentity};

use crate::config::ReplicationConfig;
use crate::error::ReplicationError;

/// Health level of a tessera's replication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationHealth {
    /// All target replicas confirmed alive.
    Healthy,
    /// Some replicas missing but above minimum threshold.
    Degraded { live: u16, target: u16 },
    /// Below minimum threshold — needs urgent repair.
    Critical { live: u16, target: u16 },
}

/// Replication status for a single tessera.
#[derive(Debug, Clone)]
pub struct TesseraReplicationStatus {
    pub tessera_hash: ContentHash,
    pub fragments_held: usize,
    pub health: ReplicationHealth,
}

/// Overall replication report.
#[derive(Debug, Clone)]
pub struct ReplicationReport {
    pub tessera_hash: ContentHash,
    pub fragments_distributed: usize,
    pub peers_contacted: usize,
    pub peers_accepted: usize,
}

/// The replication service: orchestrates fragment distribution, reception, and health checks.
pub struct ReplicationService {
    identity: NodeIdentity,
    dht: Box<dyn DhtPort>,
    fragments: Box<dyn FragmentStore>,
    ledger: Box<dyn ReciprocityLedger>,
    blobs: Box<dyn BlobStore>,
    config: ReplicationConfig,
}

impl ReplicationService {
    pub fn new(
        identity: NodeIdentity,
        dht: Box<dyn DhtPort>,
        fragments: Box<dyn FragmentStore>,
        ledger: Box<dyn ReciprocityLedger>,
        blobs: Box<dyn BlobStore>,
        config: ReplicationConfig,
    ) -> Self {
        Self {
            identity,
            dht,
            fragments,
            ledger,
            blobs,
            config,
        }
    }

    /// Receive a fragment from a remote peer. Validates checksum and reciprocity.
    pub async fn receive_fragment(
        &self,
        envelope: FragmentEnvelope,
        sender: &NodeId,
    ) -> Result<ReplicateAck, ReplicationError> {
        // 1. Verify size
        if envelope.original_tessera_size > MAX_TESSERA_SIZE {
            return Err(ReplicationError::TesseraTooBig {
                size: envelope.original_tessera_size,
                max: MAX_TESSERA_SIZE,
            });
        }

        // 2. Verify BLAKE3 checksum
        let computed = ContentHash::new(blake3::hash(&envelope.data).into());
        if computed != envelope.id.checksum {
            return Err(ReplicationError::ChecksumMismatch {
                expected: envelope.id.checksum,
                got: computed,
            });
        }

        // 3. Verify fragment_size matches data length
        if envelope.data.len() as u64 != envelope.fragment_size {
            return Err(ReplicationError::ChecksumMismatch {
                expected: envelope.id.checksum,
                got: computed,
            });
        }

        // 4. Check reciprocity balance
        let balance = self.ledger.balance(sender)?;
        if balance < -(self.config.accept_deficit_up_to_bytes as i64) {
            tracing::warn!(
                peer = %sender,
                balance,
                "rejecting fragment: reciprocity deficit too high"
            );
            return Ok(ReplicateAck {
                accepted: false,
                fragments_held: Vec::new(),
            });
        }

        // 5. Store the fragment
        self.fragments
            .store_fragment(&envelope.id, &envelope.data)?;

        // 6. Update ledger
        self.ledger
            .record_stored_for_peer(sender, envelope.data.len() as u64)?;

        // 7. Return ack with list of all fragments we hold for this tessera
        let held = self
            .fragments
            .list_fragments(&envelope.id.tessera_hash)?;
        let held_indices: Vec<u16> = held.iter().map(|f| f.index).collect();

        Ok(ReplicateAck {
            accepted: true,
            fragments_held: held_indices,
        })
    }

    /// Handle an attestation request: list local fragments and compute checksums.
    pub fn handle_attestation_request(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, ReplicationError> {
        let fragments = self.fragments.list_fragments(tessera_hash)?;

        let mut entries = Vec::with_capacity(fragments.len());
        for frag in &fragments {
            let data = self.fragments.read_fragment(frag)?;
            let checksum = ContentHash::new(blake3::hash(&data).into());
            entries.push(AttestationEntry {
                fragment_index: frag.index,
                checksum,
            });
        }

        Ok(Attestation {
            tessera_hash: *tessera_hash,
            entries,
            timestamp: chrono::Utc::now(),
            signature: Vec::new(), // signing comes in a future task
        })
    }

    /// Query local replication status for a tessera.
    pub fn status(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<TesseraReplicationStatus, ReplicationError> {
        let fragments = self.fragments.list_fragments(tessera_hash)?;
        Ok(TesseraReplicationStatus {
            tessera_hash: *tessera_hash,
            fragments_held: fragments.len(),
            health: ReplicationHealth::Healthy, // TODO: check holders when available
        })
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn dht(&self) -> &dyn DhtPort {
        &*self.dht
    }

    pub fn fragment_store(&self) -> &dyn FragmentStore {
        &*self.fragments
    }

    pub fn ledger(&self) -> &dyn ReciprocityLedger {
        &*self.ledger
    }

    pub fn blob_store(&self) -> &dyn BlobStore {
        &*self.blobs
    }

    pub fn config(&self) -> &ReplicationConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ReplicationConfig;
    use tesseras_core::replication::*;
    use tesseras_core::*;
    use mockall::mock;

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
        pub Fragments {}
        impl FragmentStore for Fragments {
            fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError>;
            fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError>;
            fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError>;
            fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError>;
            fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError>;
        }
    }

    mock! {
        pub Ledger {}
        impl ReciprocityLedger for Ledger {
            fn record_stored_for_peer(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
            fn record_peer_stores_for_us(&self, peer: &NodeId, bytes: u64) -> Result<(), CoreError>;
            fn balance(&self, peer: &NodeId) -> Result<i64, CoreError>;
            fn best_peers_for_replication(&self, count: usize) -> Result<Vec<NodeId>, CoreError>;
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

    fn hash(fill: u8) -> ContentHash {
        ContentHash::new([fill; 32])
    }
    fn node(fill: u8) -> NodeId {
        NodeId::new([fill; 20])
    }

    fn make_valid_envelope() -> FragmentEnvelope {
        let data = vec![0xaa; 64];
        let checksum = ContentHash::new(blake3::hash(&data).into());
        let plan = FragmentPlan::new(hash(0x01), 100_000_000).unwrap();
        let id = FragmentId::new(hash(0x01), 0, 16, checksum);
        FragmentEnvelope {
            id,
            plan,
            original_tessera_size: 100_000_000,
            fragment_size: 64,
            data,
        }
    }

    #[tokio::test]
    async fn receive_valid_fragment_stores_it() {
        let mut fragments = MockFragments::new();
        fragments
            .expect_store_fragment()
            .once()
            .returning(|_, _| Ok(()));
        fragments
            .expect_list_fragments()
            .once()
            .returning(|_| Ok(vec![]));

        let mut ledger = MockLedger::new();
        ledger.expect_balance().returning(|_| Ok(100));
        ledger
            .expect_record_stored_for_peer()
            .once()
            .returning(|_, _| Ok(()));

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(MockDht::new()),
            Box::new(fragments),
            Box::new(ledger),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let envelope = make_valid_envelope();
        let ack = service.receive_fragment(envelope, &node(0x01)).await.unwrap();
        assert!(ack.accepted);
    }

    #[tokio::test]
    async fn receive_fragment_rejects_bad_checksum() {
        let mut ledger = MockLedger::new();
        ledger.expect_balance().returning(|_| Ok(100));

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(MockDht::new()),
            Box::new(MockFragments::new()),
            Box::new(ledger),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let mut envelope = make_valid_envelope();
        envelope.data = vec![0xbb; 64]; // tamper with data
        let result = service.receive_fragment(envelope, &node(0x01)).await;
        assert!(matches!(
            result,
            Err(ReplicationError::ChecksumMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn receive_fragment_rejects_high_deficit() {
        let mut ledger = MockLedger::new();
        ledger.expect_balance().returning(|_| Ok(-500_000_000)); // -500 MB

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(MockDht::new()),
            Box::new(MockFragments::new()),
            Box::new(ledger),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let envelope = make_valid_envelope();
        let ack = service.receive_fragment(envelope, &node(0x01)).await.unwrap();
        assert!(!ack.accepted);
    }
}
