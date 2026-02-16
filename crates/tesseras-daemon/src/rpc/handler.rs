use std::sync::Arc;

use tesseras_core::ports::{BlobStore, FragmentStore, MemoryRepository, TesseraRepository};
use tesseras_rpc::error::ErrorCode;
use tesseras_rpc::protocol::{Request, Response};
use tesseras_replication::ReplicationService;
use tesseras_storage::CasStore;

pub struct RpcHandler {
    pub tessera_repo: Arc<dyn TesseraRepository>,
    pub memory_repo: Arc<dyn MemoryRepository>,
    pub blob_store: Arc<dyn BlobStore>,
    pub fragment_store: Arc<dyn FragmentStore>,
    pub replication: Arc<ReplicationService>,
    pub cas: Arc<CasStore>,
}

impl RpcHandler {
    pub async fn handle(&self, request: Request) -> Response {
        match request {
            Request::Publish { hash } => self.handle_publish(hash).await,
            Request::Fetch { hash } => self.handle_fetch(hash).await,
            Request::Status { hash } => self.handle_status(hash).await,
        }
    }

    async fn handle_publish(&self, hash: tesseras_core::ContentHash) -> Response {
        // 1. Pack tessera from storage
        let packed = match crate::rpc::pack::pack_tessera(
            &hash,
            self.tessera_repo.as_ref(),
            self.memory_repo.as_ref(),
            self.blob_store.as_ref(),
        ) {
            Ok(data) => data,
            Err(e) => {
                let msg = e.to_string();
                let code = if msg.contains("not found") {
                    ErrorCode::NotFound
                } else {
                    ErrorCode::Internal
                };
                return Response::Error {
                    code,
                    message: msg,
                };
            }
        };

        // 2. Replicate to network
        match self.replication.replicate_tessera(&hash, &packed).await {
            Ok(report) => Response::Published {
                hash,
                fragments_created: report.fragments_distributed as u32,
            },
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("replication failed: {e}"),
            },
        }
    }

    async fn handle_fetch(&self, hash: tesseras_core::ContentHash) -> Response {
        // Fetch from network/local fragments
        let data = match self.replication.fetch_tessera(&hash).await {
            Ok(d) => d,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::NotFound,
                    message: format!("failed to fetch tessera: {e}"),
                };
            }
        };

        // Unpack and store into local storage
        let files = match tesseras_core::pack::unpack(&data) {
            Ok(f) => f,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("failed to unpack tessera: {e}"),
                };
            }
        };

        let mut total_bytes: u64 = 0;
        let mut seen_memories = std::collections::HashSet::new();

        for file in &files {
            total_bytes += file.data.len() as u64;
            // Track unique memory directories
            if let Some(rest) = file.path.strip_prefix("memories/") {
                if let Some(mem_hash) = rest.split('/').next() {
                    seen_memories.insert(mem_hash.to_string());
                }
            }
        }
        let memory_count = seen_memories.len() as u32;

        // Store blobs into CAS
        for file in &files {
            if let Err(e) = self.cas.put(&file.data) {
                tracing::warn!(path = %file.path, error = %e, "failed to store fetched blob");
            }
        }

        Response::Fetched {
            hash,
            memories: memory_count,
            bytes: total_bytes,
        }
    }

    async fn handle_status(&self, hash: tesseras_core::ContentHash) -> Response {
        // Check if tessera exists locally
        let exists = match self.tessera_repo.exists(&hash) {
            Ok(e) => e,
            Err(e) => {
                return Response::Error {
                    code: ErrorCode::Internal,
                    message: format!("storage error: {e}"),
                };
            }
        };

        if !exists {
            return Response::Error {
                code: ErrorCode::NotFound,
                message: format!("tessera not found: {hash}"),
            };
        }

        // Query replication status
        match self.replication.status(&hash) {
            Ok(status) => {
                let (state, fragments_total, fragments_placed) = match &status.health {
                    tesseras_replication::ReplicationHealth::Healthy => (
                        tesseras_rpc::PublishState::Healthy,
                        status.fragments_held as u32,
                        status.fragments_held as u32,
                    ),
                    tesseras_replication::ReplicationHealth::Degraded { live, target } => (
                        tesseras_rpc::PublishState::Replicated,
                        *target as u32,
                        *live as u32,
                    ),
                    tesseras_replication::ReplicationHealth::Critical { live, target } => (
                        tesseras_rpc::PublishState::Publishing,
                        *target as u32,
                        *live as u32,
                    ),
                };

                // If no fragments at all, it's still local-only
                let state = if status.fragments_held == 0 {
                    tesseras_rpc::PublishState::Local
                } else {
                    state
                };

                Response::Status {
                    hash,
                    state,
                    fragments_total,
                    fragments_placed,
                    peers_holding: 0, // TODO: peer count from DHT
                }
            }
            Err(e) => Response::Error {
                code: ErrorCode::Internal,
                message: format!("replication status error: {e}"),
            },
        }
    }
}
