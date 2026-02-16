use tesseras_core::ports::{BlobStore, DhtPort, FragmentStore, ReciprocityLedger};
use tesseras_core::replication::{
    Attestation, AttestationEntry, FragmentEnvelope, FragmentId, FragmentationTier,
    MAX_TESSERA_SIZE, ReplicateAck,
};
use tesseras_core::types::NodeId;
use tesseras_core::{ContentHash, NodeIdentity};

use crate::config::ReplicationConfig;
use crate::distributor::apply_subnet_diversity;
use crate::error::ReplicationError;
use crate::fragment::encode_tessera;

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
    cas: Option<std::sync::Arc<tesseras_storage::CasStore>>,
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
            cas: None,
        }
    }

    pub fn with_cas(mut self, cas: std::sync::Arc<tesseras_storage::CasStore>) -> Self {
        self.cas = Some(cas);
        self
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

        // 4. Check reciprocity balance (skip for institutional peers)
        let is_institutional = self.ledger.is_institutional(sender)?;
        if !is_institutional {
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
        }

        // 5. Store the fragment
        self.fragments
            .store_fragment(&envelope.id, &envelope.data)?;

        // 6. Update ledger
        self.ledger
            .record_stored_for_peer(sender, envelope.data.len() as u64)?;

        // 7. Return ack with list of all fragments we hold for this tessera
        let held = self.fragments.list_fragments(&envelope.id.tessera_hash)?;
        let held_indices: Vec<u16> = held.iter().map(|f| f.index).collect();

        Ok(ReplicateAck {
            accepted: true,
            fragments_held: held_indices,
        })
    }

    /// Handle an attestation request: list local fragments and return stored checksums.
    pub fn handle_attestation_request(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, ReplicationError> {
        let fragments = self.fragments.list_fragments(tessera_hash)?;

        let mut entries = Vec::with_capacity(fragments.len());
        for frag in &fragments {
            entries.push(AttestationEntry {
                fragment_index: frag.index,
                checksum: frag.checksum,
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

    /// Fetch a tessera from local fragments. For small tier, the single fragment
    /// IS the tessera data. For medium/large, erasure-decode from available shards.
    pub async fn fetch_tessera(
        &self,
        tessera_hash: &ContentHash,
    ) -> Result<Vec<u8>, ReplicationError> {
        let local_fragments = self.fragments.list_fragments(tessera_hash)?;

        if local_fragments.is_empty() {
            // TODO: query DHT for remote fragments in a future iteration
            return Err(ReplicationError::NoFragmentsAvailable {
                tessera_hash: *tessera_hash,
            });
        }

        // For small tier: single fragment (index=0, not parity) IS the tessera data
        let first = &local_fragments[0];
        if local_fragments.len() == 1 && first.index == 0 && !first.is_parity {
            let data = self.fragments.read_fragment(first)?;
            return Ok(data);
        }

        // For medium/large tier: collect all fragments and erasure-decode
        let data_count = local_fragments.iter().filter(|f| !f.is_parity).count();
        let parity_count = local_fragments.iter().filter(|f| f.is_parity).count();
        let total = data_count + parity_count;

        let mut shards: Vec<Option<tesseras_crypto::erasure::Fragment>> = vec![None; total];
        for frag_id in &local_fragments {
            let data = self.fragments.read_fragment(frag_id)?;
            shards[frag_id.index as usize] = Some(tesseras_crypto::erasure::Fragment {
                index: frag_id.index as usize,
                data,
            });
        }

        tesseras_crypto::erasure::ReedSolomonCoder::decode(&shards, data_count, parity_count)
            .map_err(|e| ReplicationError::ErasureCoding(e.to_string()))
    }

    /// Replicate a tessera to the network.
    ///
    /// Small tier: push whole file to r peers.
    /// Medium/Large tier: erasure-code, store fragments locally, push to peers.
    pub async fn replicate_tessera(
        &self,
        tessera_hash: &ContentHash,
        tessera_data: &[u8],
    ) -> Result<ReplicationReport, ReplicationError> {
        let encoded = encode_tessera(tessera_hash, tessera_data)?;

        // Find target peers
        let mut target_bytes = [0u8; 20];
        target_bytes.copy_from_slice(&tessera_hash.as_bytes()[..20]);
        let target_node = NodeId::new(target_bytes);
        let candidates = self.dht.find_closest_nodes(&target_node).await;
        let replication_factor = match &encoded.plan.tier {
            FragmentationTier::Small {
                replication_factor, ..
            }
            | FragmentationTier::Medium {
                replication_factor, ..
            }
            | FragmentationTier::Large {
                replication_factor, ..
            } => *replication_factor as usize,
        };

        let peers = apply_subnet_diversity(&candidates, 2);
        let peers = crate::distributor::apply_institutional_diversity(&peers, replication_factor);

        let target_peers = &peers[..peers.len().min(replication_factor)];
        let mut peers_contacted = 0;
        let mut peers_accepted = 0;
        let mut fragments_distributed = 0;

        match &encoded.plan.tier {
            FragmentationTier::Small { .. } => {
                // Push whole file to each peer
                let checksum = ContentHash::new(blake3::hash(tessera_data).into());
                let id = FragmentId::new(*tessera_hash, 0, 1, checksum);

                // Store fragment locally so status() can track it
                self.fragments.store_fragment(&id, tessera_data)?;

                let envelope = FragmentEnvelope {
                    id,
                    plan: encoded.plan.clone(),
                    original_tessera_size: tessera_data.len() as u64,
                    fragment_size: tessera_data.len() as u64,
                    data: tessera_data.to_vec(),
                };

                for peer in target_peers {
                    peers_contacted += 1;
                    match self.dht.replicate_fragment(peer, &envelope).await {
                        Ok(ack) if ack.accepted => {
                            peers_accepted += 1;
                            fragments_distributed += 1;
                            self.ledger.record_peer_stores_for_us(
                                &peer.identity.node_id,
                                tessera_data.len() as u64,
                            )?;
                        }
                        Ok(_) => {
                            tracing::debug!(
                                peer = %peer.identity.node_id,
                                "peer rejected fragment"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                peer = %peer.identity.node_id,
                                error = %e,
                                "failed to replicate to peer"
                            );
                        }
                    }
                }
            }
            FragmentationTier::Medium { .. } | FragmentationTier::Large { .. } => {
                // Store fragments locally first
                for (id, frag_data) in &encoded.fragments {
                    self.fragments.store_fragment(id, frag_data)?;
                }

                // Distribute fragments across peers (round-robin)
                for (i, (id, frag_data)) in encoded.fragments.iter().enumerate() {
                    let peer_idx = i % target_peers.len().max(1);
                    if peer_idx >= target_peers.len() {
                        continue;
                    }
                    let peer = &target_peers[peer_idx];

                    let envelope = FragmentEnvelope {
                        id: id.clone(),
                        plan: encoded.plan.clone(),
                        original_tessera_size: tessera_data.len() as u64,
                        fragment_size: frag_data.len() as u64,
                        data: frag_data.clone(),
                    };

                    peers_contacted += 1;
                    match self.dht.replicate_fragment(peer, &envelope).await {
                        Ok(ack) if ack.accepted => {
                            peers_accepted += 1;
                            fragments_distributed += 1;
                            self.ledger.record_peer_stores_for_us(
                                &peer.identity.node_id,
                                frag_data.len() as u64,
                            )?;
                        }
                        Ok(_) => {
                            tracing::debug!(
                                peer = %peer.identity.node_id,
                                fragment = id.index,
                                "peer rejected fragment"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                peer = %peer.identity.node_id,
                                fragment = id.index,
                                error = %e,
                                "failed to replicate fragment"
                            );
                        }
                    }
                }
            }
        }

        if peers_accepted == 0 && !target_peers.is_empty() {
            return Err(ReplicationError::NoPeersAvailable);
        }

        Ok(ReplicationReport {
            tessera_hash: *tessera_hash,
            fragments_distributed,
            peers_contacted,
            peers_accepted,
        })
    }

    /// Run the repair loop: periodically check all tesseras and repair degraded ones.
    pub async fn run_repair_loop(&self, mut shutdown: tokio::sync::watch::Receiver<bool>) {
        loop {
            let jitter = {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                std::time::Duration::from_secs(
                    rng.gen_range(0..self.config.repair_jitter.as_secs().max(1)),
                )
            };
            let sleep_time = self.config.repair_interval + jitter;

            tokio::select! {
                _ = tokio::time::sleep(sleep_time) => {
                    tracing::info!("starting repair sweep");
                    self.run_repair_sweep().await;
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("repair loop shutting down");
                        break;
                    }
                }
            }
        }
    }

    /// Execute one repair sweep over all locally tracked tesseras.
    async fn run_repair_sweep(&self) {
        // Run CAS dedup sweep if available
        if let Some(ref cas) = self.cas {
            match cas.sweep() {
                Ok(stats) => {
                    tracing::info!(
                        orphans_removed = stats.orphan_files_removed,
                        orphans_skipped_young = stats.orphan_files_skipped_young,
                        leaked_refs_removed = stats.leaked_refs_removed,
                        "CAS sweep complete"
                    );
                }
                Err(e) => {
                    tracing::error!(error = %e, "CAS sweep failed");
                }
            }
        }
        tracing::info!("repair sweep complete");
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
    use mockall::mock;
    use std::net::SocketAddr;
    use tesseras_core::replication::{FragmentId, FragmentPlan};
    use tesseras_core::*;

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
            fn mark_institutional(&self, peer: &NodeId) -> Result<(), CoreError>;
            fn is_institutional(&self, peer: &NodeId) -> Result<bool, CoreError>;
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
        ledger.expect_is_institutional().returning(|_| Ok(false));
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
        let ack = service
            .receive_fragment(envelope, &node(0x01))
            .await
            .unwrap();
        assert!(ack.accepted);
    }

    #[tokio::test]
    async fn receive_fragment_rejects_bad_checksum() {
        let ledger = MockLedger::new();

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
        ledger.expect_is_institutional().returning(|_| Ok(false));
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
        let ack = service
            .receive_fragment(envelope, &node(0x01))
            .await
            .unwrap();
        assert!(!ack.accepted);
    }

    #[tokio::test]
    async fn receive_fragment_accepts_from_institutional_despite_deficit() {
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
        ledger.expect_is_institutional().returning(|_| Ok(true));
        // balance() should NOT be called when institutional
        ledger.expect_balance().never();
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
        let ack = service
            .receive_fragment(envelope, &node(0x01))
            .await
            .unwrap();
        assert!(ack.accepted);
    }

    fn make_node_info(fill: u8, port: u16) -> NodeInfo {
        NodeInfo {
            identity: NodeIdentity {
                node_id: node(fill),
                public_key: [fill; 32],
                nonce: 0,
            },
            addr: SocketAddr::from(([10, 0, fill, 1], port)),
            alt_addrs: vec![],
            capabilities: Capabilities::phase2_default(),
        }
    }

    #[tokio::test]
    async fn replicate_small_tessera_pushes_to_r_peers() {
        let mut dht = MockDht::new();
        let peers: Vec<NodeInfo> = (1..=7).map(|i| make_node_info(i, 4433)).collect();
        let peers_clone = peers.clone();
        dht.expect_find_closest_nodes()
            .once()
            .returning(move |_| peers_clone.clone());
        dht.expect_replicate_fragment().times(7).returning(|_, _| {
            Ok(ReplicateAck {
                accepted: true,
                fragments_held: vec![],
            })
        });

        let mut ledger = MockLedger::new();
        ledger
            .expect_record_peer_stores_for_us()
            .times(7)
            .returning(|_, _| Ok(()));

        let mut fragments = MockFragments::new();
        fragments
            .expect_store_fragment()
            .once()
            .returning(|_, _| Ok(()));

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(dht),
            Box::new(fragments),
            Box::new(ledger),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let data = vec![0xaa; 1000]; // 1KB — small tier
        let tessera_hash = hash(0x01);
        let report = service
            .replicate_tessera(&tessera_hash, &data)
            .await
            .unwrap();
        assert_eq!(report.peers_accepted, 7);
        assert_eq!(report.fragments_distributed, 7);
    }

    #[tokio::test]
    async fn replicate_medium_tessera_encodes_and_distributes() {
        let mut dht = MockDht::new();
        let peers: Vec<NodeInfo> = (1..=7).map(|i| make_node_info(i, 4433)).collect();
        let peers_clone = peers.clone();
        dht.expect_find_closest_nodes()
            .once()
            .returning(move |_| peers_clone.clone());
        dht.expect_replicate_fragment()
            .times(24) // 16 data + 8 parity
            .returning(|_, _| {
                Ok(ReplicateAck {
                    accepted: true,
                    fragments_held: vec![],
                })
            });

        let mut fragments = MockFragments::new();
        fragments
            .expect_store_fragment()
            .times(24)
            .returning(|_, _| Ok(()));

        let mut ledger = MockLedger::new();
        ledger
            .expect_record_peer_stores_for_us()
            .times(24)
            .returning(|_, _| Ok(()));

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(dht),
            Box::new(fragments),
            Box::new(ledger),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let data = vec![0xbb; 10 * 1024 * 1024]; // 10 MB — medium tier
        let tessera_hash = hash(0x02);
        let report = service
            .replicate_tessera(&tessera_hash, &data)
            .await
            .unwrap();
        assert_eq!(report.fragments_distributed, 24);
    }

    #[tokio::test]
    async fn attestation_uses_stored_checksums_not_blob_reads() {
        let dht = MockDht::new();
        let mut fragments = MockFragments::new();
        let tessera_hash = hash(0xAA);
        let checksum_a = hash(0x11);
        let checksum_b = hash(0x22);

        let hash_clone = tessera_hash;
        fragments.expect_list_fragments().returning(move |h| {
            assert_eq!(*h, hash_clone);
            Ok(vec![
                FragmentId::new(hash_clone, 0, 16, checksum_a),
                FragmentId::new(hash_clone, 1, 16, checksum_b),
            ])
        });

        // read_fragment should NEVER be called for attestation
        fragments.expect_read_fragment().never();

        let ledger = MockLedger::new();
        let blobs = MockBlobs::new();

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(1),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(dht),
            Box::new(fragments),
            Box::new(ledger),
            Box::new(blobs),
            ReplicationConfig::default(),
        );

        let attestation = service.handle_attestation_request(&tessera_hash).unwrap();
        assert_eq!(attestation.entries.len(), 2);
        assert_eq!(attestation.entries[0].fragment_index, 0);
        assert_eq!(attestation.entries[0].checksum, checksum_a);
        assert_eq!(attestation.entries[1].fragment_index, 1);
        assert_eq!(attestation.entries[1].checksum, checksum_b);
    }

    #[tokio::test]
    async fn fetch_tessera_from_local_small_fragment() {
        let tessera_hash = hash(0x10);
        let data = vec![0xcc; 500];
        let checksum = ContentHash::new(blake3::hash(&data).into());
        let frag_id = FragmentId::new(tessera_hash, 0, 1, checksum);

        let mut fragments = MockFragments::new();
        let frag_id_clone = frag_id.clone();
        fragments.expect_list_fragments().returning(move |_| {
            Ok(vec![frag_id_clone.clone()])
        });
        let data_clone = data.clone();
        fragments.expect_read_fragment().returning(move |_| {
            Ok(data_clone.clone())
        });

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(MockDht::new()),
            Box::new(fragments),
            Box::new(MockLedger::new()),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let fetched = service.fetch_tessera(&tessera_hash).await.unwrap();
        assert_eq!(fetched, data);
    }

    #[tokio::test]
    async fn fetch_tessera_returns_error_when_no_fragments() {
        let mut fragments = MockFragments::new();
        fragments.expect_list_fragments().returning(|_| Ok(vec![]));

        let service = ReplicationService::new(
            NodeIdentity {
                node_id: node(0xff),
                public_key: [0; 32],
                nonce: 0,
            },
            Box::new(MockDht::new()),
            Box::new(fragments),
            Box::new(MockLedger::new()),
            Box::new(MockBlobs::new()),
            ReplicationConfig::default(),
        );

        let result = service.fetch_tessera(&hash(0x20)).await;
        assert!(matches!(
            result,
            Err(ReplicationError::NoFragmentsAvailable { .. })
        ));
    }
}
