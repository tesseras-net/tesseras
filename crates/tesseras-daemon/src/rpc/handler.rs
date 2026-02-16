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

    async fn handle_fetch(&self, _hash: tesseras_core::ContentHash) -> Response {
        // TODO: implement in Task 9
        Response::Error {
            code: ErrorCode::Internal,
            message: "fetch not yet implemented".into(),
        }
    }

    async fn handle_status(&self, _hash: tesseras_core::ContentHash) -> Response {
        // TODO: implement in Task 9
        Response::Error {
            code: ErrorCode::Internal,
            message: "status not yet implemented".into(),
        }
    }
}
