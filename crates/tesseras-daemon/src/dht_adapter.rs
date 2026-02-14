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
        self.engine
            .replicate_fragment(target, fragment)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))
    }

    async fn request_attestation(
        &self,
        target: &NodeInfo,
        tessera_hash: &ContentHash,
    ) -> Result<Attestation, CoreError> {
        self.engine
            .request_attestation(target, tessera_hash)
            .await
            .map_err(|e| CoreError::Network(e.to_string()))
    }

    async fn ping(&self, target: &NodeInfo) -> bool {
        self.engine.ping(target.addr).await
    }
}
