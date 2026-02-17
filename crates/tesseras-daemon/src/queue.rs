use std::sync::Arc;
use std::time::Duration;

use tesseras_core::ports::QueuedOperation;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::rpc::handler::RpcHandler;

const MAX_RETRIES: u32 = 10;

/// Polls the operation queue and processes pending operations.
pub async fn run_queue_processor(handler: Arc<RpcHandler>, mut shutdown: watch::Receiver<bool>) {
    let base_interval = Duration::from_secs(30);
    let max_interval = Duration::from_secs(3600);
    let mut interval = base_interval;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(interval) => {
                match handler.operation_queue.dequeue_pending() {
                    Ok(Some(entry)) => {
                        info!(id = entry.id, op = ?entry.operation, "processing queued operation");
                        let result = process_operation(&handler, &entry.operation).await;
                        match result {
                            Ok(()) => {
                                if let Err(e) = handler.operation_queue.mark_completed(entry.id) {
                                    warn!(id = entry.id, error = %e, "failed to mark operation completed");
                                }
                            }
                            Err(e) => {
                                warn!(id = entry.id, error = %e, "queued operation failed");
                                let _ = handler.operation_queue.increment_retries(entry.id);
                                if entry.retries + 1 >= MAX_RETRIES {
                                    let _ = handler.operation_queue.mark_failed(entry.id, &e);
                                }
                            }
                        }
                        interval = base_interval; // reset on activity
                    }
                    Ok(None) => {
                        // No pending work, back off
                        interval = (interval * 2).min(max_interval);
                    }
                    Err(e) => {
                        warn!("queue poll error: {e}");
                    }
                }
            }
            _ = shutdown.changed() => {
                info!("queue processor shutting down");
                break;
            }
        }
    }
}

async fn process_operation(handler: &RpcHandler, op: &QueuedOperation) -> Result<(), String> {
    match op {
        QueuedOperation::Push { hash } => {
            let resp = handler
                .handle(tesseras_rpc::Request::Publish { hash: *hash })
                .await;
            response_to_result(resp)
        }
        QueuedOperation::Pull { hash } => {
            let resp = handler
                .handle(tesseras_rpc::Request::Fetch { hash: *hash })
                .await;
            response_to_result(resp)
        }
        QueuedOperation::Delete { hash } => {
            let resp = handler
                .handle(tesseras_rpc::Request::Delete {
                    hash: hash.to_string(),
                })
                .await;
            response_to_result(resp)
        }
        QueuedOperation::Retract { hash } => {
            // Retraction is handled as part of Delete
            let resp = handler
                .handle(tesseras_rpc::Request::Delete {
                    hash: hash.to_string(),
                })
                .await;
            response_to_result(resp)
        }
    }
}

fn response_to_result(resp: tesseras_rpc::Response) -> Result<(), String> {
    match resp {
        tesseras_rpc::Response::Error { message, .. } => Err(message),
        _ => Ok(()),
    }
}
