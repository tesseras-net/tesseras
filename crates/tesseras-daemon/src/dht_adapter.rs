//! Adapter that wraps DhtEngine to implement the DhtPort trait.

use std::sync::Arc;

use tesseras_core::ports::DhtPort;
use tesseras_core::replication::{Attestation, FragmentEnvelope, ReplicateAck};
use tesseras_core::types::NodeId;
use tesseras_core::{ContentHash, CoreError, NodeInfo};
use tesseras_dht::engine::DhtEngine;

pub struct DhtPortAdapter {
    engine: Arc<DhtEngine>,
}

impl DhtPortAdapter {
    pub fn new(engine: Arc<DhtEngine>) -> Self {
        Self { engine }
    }
}

#[async_trait::async_trait]
impl DhtPort for DhtPortAdapter {
    async fn find_closest_nodes(&self, target: &NodeId) -> Vec<NodeInfo> {
        self.engine.find_closest_nodes(target).await
    }

    async fn replicate_fragment(
        &self,
        target: &NodeInfo,
        fragment: &FragmentEnvelope,
    ) -> Result<ReplicateAck, CoreError> {
        // Construct and send a REPLICATE message via DHT engine.
        // For now, this is a stub — full wire integration requires
        // access to the engine's private rpc() method.
        // In practice, REPLICATE would be sent as a QUIC stream.
        tracing::debug!(
            target_node = %target.identity.node_id,
            tessera = %fragment.id.tessera_hash,
            fragment = fragment.id.index,
            "replicate_fragment (adapter stub)"
        );
        Ok(ReplicateAck {
            accepted: true,
            fragments_held: vec![fragment.id.index],
        })
    }

    async fn request_attestation(
        &self,
        target: &NodeInfo,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, CoreError> {
        tracing::debug!(
            target_node = %target.identity.node_id,
            tessera = %tessera_hash,
            "request_attestation (adapter stub)"
        );
        Ok(Attestation {
            tessera_hash: *tessera_hash,
            entries: vec![],
            timestamp: chrono::Utc::now(),
            signature: vec![],
        })
    }

    async fn ping(&self, target: &NodeInfo) -> bool {
        self.engine.ping(target.addr).await
    }
}
