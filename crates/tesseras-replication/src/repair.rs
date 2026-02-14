use tesseras_core::ports::{DhtPort, FragmentStore};
use tesseras_core::{ContentHash, NodeInfo};

/// Outcome of checking a tessera's replication health.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairAction {
    /// All holders confirmed alive, all fragments valid.
    Healthy,
    /// Some holders unreachable — need additional replicas.
    NeedsReplication { deficit: u16 },
    /// A local fragment has corrupt data.
    CorruptLocal { fragment_index: u16 },
}

/// Check health of a single tessera by requesting attestations from known holders.
pub async fn check_tessera_health(
    dht: &dyn DhtPort,
    fragment_store: &dyn FragmentStore,
    tessera_hash: &ContentHash,
    holders: &[NodeInfo],
    target_replicas: u16,
) -> RepairAction {
    let mut live_count: u16 = 0;

    // Check remote holders via attestation
    for holder in holders {
        match dht.request_attestation(holder, tessera_hash).await {
            Ok(_attestation) => {
                live_count += 1;
            }
            Err(_) => {
                // Attestation failed, try ping as fallback
                if dht.ping(holder).await {
                    live_count += 1;
                }
            }
        }
    }

    // Check local fragments for corruption
    let local_fragments = match fragment_store.list_fragments(tessera_hash) {
        Ok(frags) => frags,
        Err(_) => return RepairAction::Healthy, // no local fragments = not our concern
    };

    for frag in &local_fragments {
        match fragment_store.verify_fragment(frag) {
            Ok(true) => {}
            Ok(false) => {
                return RepairAction::CorruptLocal {
                    fragment_index: frag.index,
                };
            }
            Err(_) => {
                return RepairAction::CorruptLocal {
                    fragment_index: frag.index,
                };
            }
        }
    }

    // Count ourselves if we hold fragments
    if !local_fragments.is_empty() {
        live_count += 1;
    }

    if live_count >= target_replicas {
        RepairAction::Healthy
    } else {
        RepairAction::NeedsReplication {
            deficit: target_replicas - live_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use tesseras_core::replication::{
        Attestation, FragmentEnvelope, FragmentId, ReplicateAck,
    };
    use tesseras_core::types::NodeId;
    use tesseras_core::{Capabilities, CoreError, NodeIdentity};

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
        pub Fragments {}
        impl FragmentStore for Fragments {
            fn store_fragment(&self, id: &FragmentId, data: &[u8]) -> Result<(), CoreError>;
            fn read_fragment(&self, id: &FragmentId) -> Result<Vec<u8>, CoreError>;
            fn delete_fragment(&self, id: &FragmentId) -> Result<(), CoreError>;
            fn list_fragments(&self, tessera_hash: &ContentHash) -> Result<Vec<FragmentId>, CoreError>;
            fn verify_fragment(&self, id: &FragmentId) -> Result<bool, CoreError>;
        }
    }

    fn make_holder(fill: u8) -> NodeInfo {
        NodeInfo {
            identity: NodeIdentity {
                node_id: node(fill),
                public_key: [fill; 32],
                nonce: 0,
            },
            addr: std::net::SocketAddr::from(([10, 0, 0, fill], 4433)),
            capabilities: Capabilities::phase2_default(),
        }
    }

    #[tokio::test]
    async fn repair_check_identifies_degraded_tessera() {
        let mut dht = MockDht::new();
        // Only 3 of 7 holders respond
        dht.expect_request_attestation()
            .times(7)
            .returning(|holder, _| {
                let fill = holder.identity.public_key[0];
                if fill <= 3 {
                    Ok(Attestation {
                        tessera_hash: ContentHash::new([0x01; 32]),
                        entries: vec![],
                        timestamp: chrono::Utc::now(),
                        signature: vec![],
                    })
                } else {
                    Err(CoreError::Io(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "timeout",
                    )))
                }
            });
        // Ping fallback also fails for 4-7
        dht.expect_ping()
            .returning(|_| false);

        let mut fragments = MockFragments::new();
        fragments
            .expect_list_fragments()
            .returning(|_| Ok(vec![]));

        let holders: Vec<NodeInfo> = (1..=7).map(make_holder).collect();

        let action =
            check_tessera_health(&dht, &fragments, &hash(0x01), &holders, 7).await;

        assert!(matches!(
            action,
            RepairAction::NeedsReplication { deficit: 4 }
        ));
    }

    #[tokio::test]
    async fn repair_check_healthy_tessera_is_noop() {
        let mut dht = MockDht::new();
        dht.expect_request_attestation()
            .times(7)
            .returning(|_, _| {
                Ok(Attestation {
                    tessera_hash: ContentHash::new([0x01; 32]),
                    entries: vec![],
                    timestamp: chrono::Utc::now(),
                    signature: vec![],
                })
            });

        let mut fragments = MockFragments::new();
        fragments
            .expect_list_fragments()
            .returning(|_| Ok(vec![]));

        let holders: Vec<NodeInfo> = (1..=7).map(make_holder).collect();

        let action =
            check_tessera_health(&dht, &fragments, &hash(0x01), &holders, 7).await;

        assert_eq!(action, RepairAction::Healthy);
    }

    #[tokio::test]
    async fn repair_detects_corrupt_local_fragment() {
        let dht = MockDht::new();
        // No remote holders
        let mut fragments = MockFragments::new();
        let frag_id = FragmentId::new(hash(0x01), 5, 16, hash(0xaa));
        let frag_id_clone = frag_id.clone();
        fragments
            .expect_list_fragments()
            .returning(move |_| Ok(vec![frag_id_clone.clone()]));
        fragments
            .expect_verify_fragment()
            .returning(|_| Ok(false)); // corrupt!

        let action =
            check_tessera_health(&dht, &fragments, &hash(0x01), &[], 7).await;

        assert_eq!(
            action,
            RepairAction::CorruptLocal {
                fragment_index: 5
            }
        );
    }
}
