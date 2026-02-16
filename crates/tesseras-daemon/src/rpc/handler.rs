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

    async fn handle_publish(&self, _hash: tesseras_core::ContentHash) -> Response {
        // TODO: implement in Task 7
        Response::Error {
            code: ErrorCode::Internal,
            message: "publish not yet implemented".into(),
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
