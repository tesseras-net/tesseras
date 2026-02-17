//! Unix socket RPC for communication between `tes` CLI and the daemon.
//!
//! Protocol: length-prefixed MessagePack over a Unix stream socket.
//! Each message is preceded by a 4-byte big-endian length prefix.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, info, warn};

use crate::types::{ContentHash, Tessera, Visibility};

/// Default socket filename within the data directory.
const SOCKET_FILENAME: &str = "daemon.sock";

/// Maximum RPC message size (16 MB).
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// RPC request from CLI to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcRequest {
    /// Ping the daemon (health check).
    Ping,
    /// Add a tessera from files already present on disk.
    AddTessera {
        files: Vec<PathBuf>,
        name: Option<String>,
        visibility: Visibility,
    },
    /// Remove a tessera by hash.
    RemoveTessera { hash: ContentHash },
    /// List all tesseras.
    ListTesseras,
    /// Get a tessera by hash.
    GetTessera { hash: ContentHash },
    /// Get node status (peer count, tessera count, etc).
    NodeStatus,
    /// Check fragment health.
    CheckFragments,
    /// Fetch a tessera from the network (DHT lookup + fragment reconstruction).
    FetchTesseraFromNetwork { hash: ContentHash },
}

/// RPC response from daemon to CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcResponse {
    /// Simple acknowledgement.
    Ok,
    /// Error message.
    Error(String),
    /// Pong response with node info.
    Pong {
        node_id: String,
        peer_count: usize,
        listen_addr: String,
    },
    /// A single tessera.
    Tessera(Tessera),
    /// A list of tesseras.
    TesseraList(Vec<Tessera>),
    /// Node status.
    Status {
        node_id: String,
        peer_count: usize,
        tessera_count: usize,
        listen_addr: String,
    },
    /// Fragment health report.
    FragmentHealth {
        total_ok: usize,
        missing: Vec<(String, usize, String)>, // (blob_hash, index, frag_hash)
    },
}

/// Get the socket path for a data directory.
pub fn socket_path(data_dir: &Path) -> PathBuf {
    data_dir.join(SOCKET_FILENAME)
}

/// Check if the daemon socket exists and is connectable.
pub async fn daemon_is_running(data_dir: &Path) -> bool {
    let path = socket_path(data_dir);
    if !path.exists() {
        return false;
    }
    UnixStream::connect(&path).await.is_ok()
}

// --- Client side ---

/// Send an RPC request to the daemon and receive the response.
pub async fn send_request(data_dir: &Path, request: &RpcRequest) -> Result<RpcResponse, RpcError> {
    let path = socket_path(data_dir);
    let mut stream = UnixStream::connect(&path)
        .await
        .map_err(|e| RpcError::Connect(e.to_string()))?;

    write_message(&mut stream, request).await?;
    read_message(&mut stream).await
}

// --- Server side ---

/// RPC server that listens on a Unix socket and dispatches to a handler.
pub struct RpcServer {
    listener: UnixListener,
    socket_path: PathBuf,
}

impl RpcServer {
    /// Bind the RPC server to the socket path in the data directory.
    pub fn bind(data_dir: &Path) -> Result<Self, RpcError> {
        let path = socket_path(data_dir);

        // Remove stale socket if present
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let listener = UnixListener::bind(&path).map_err(|e| RpcError::Bind(e.to_string()))?;

        info!("RPC listening on {}", path.display());
        Ok(Self {
            listener,
            socket_path: path,
        })
    }

    /// Accept connections and dispatch to the handler function.
    /// Runs until the shutdown signal fires.
    pub async fn serve<F, Fut>(self, handler: F, mut shutdown: tokio::sync::watch::Receiver<bool>)
    where
        F: Fn(RpcRequest) -> Fut + Send + Sync + 'static + Clone,
        Fut: std::future::Future<Output = RpcResponse> + Send,
    {
        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            let handler = handler.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, handler).await {
                                    debug!("RPC client error: {e}");
                                }
                            });
                        }
                        Err(e) => {
                            warn!("RPC accept error: {e}");
                        }
                    }
                }
                _ = shutdown.changed() => {
                    info!("RPC server shutting down");
                    break;
                }
            }
        }

        // Clean up socket file
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Handle a single RPC client connection.
async fn handle_client<F, Fut>(mut stream: UnixStream, handler: F) -> Result<(), RpcError>
where
    F: Fn(RpcRequest) -> Fut + Send,
    Fut: std::future::Future<Output = RpcResponse> + Send,
{
    let request: RpcRequest = read_message(&mut stream).await?;
    let response = handler(request).await;
    write_message(&mut stream, &response).await?;
    Ok(())
}

// --- Wire format ---

/// Write a length-prefixed MessagePack message.
async fn write_message<T: Serialize>(stream: &mut UnixStream, msg: &T) -> Result<(), RpcError> {
    let data = rmp_serde::to_vec(msg).map_err(|e| RpcError::Serialize(e.to_string()))?;
    let len = (data.len() as u32).to_be_bytes();
    stream
        .write_all(&len)
        .await
        .map_err(|e| RpcError::Io(e.to_string()))?;
    stream
        .write_all(&data)
        .await
        .map_err(|e| RpcError::Io(e.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|e| RpcError::Io(e.to_string()))?;
    Ok(())
}

/// Read a length-prefixed MessagePack message.
async fn read_message<T: for<'de> Deserialize<'de>>(
    stream: &mut UnixStream,
) -> Result<T, RpcError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| RpcError::Io(e.to_string()))?;
    let len = u32::from_be_bytes(len_buf) as usize;

    if len > MAX_MESSAGE_SIZE {
        return Err(RpcError::MessageTooLarge(len));
    }

    let mut data = vec![0u8; len];
    stream
        .read_exact(&mut data)
        .await
        .map_err(|e| RpcError::Io(e.to_string()))?;

    rmp_serde::from_slice(&data).map_err(|e| RpcError::Deserialize(e.to_string()))
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("connect error: {0}")]
    Connect(String),
    #[error("bind error: {0}")]
    Bind(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("serialize error: {0}")]
    Serialize(String),
    #[error("deserialize error: {0}")]
    Deserialize(String),
    #[error("message too large: {0} bytes")]
    MessageTooLarge(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rpc_ping_pong() {
        let tmp = tempfile::tempdir().unwrap();

        let server = RpcServer::bind(tmp.path()).unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            server
                .serve(
                    |req| async move {
                        match req {
                            RpcRequest::Ping => RpcResponse::Pong {
                                node_id: "test-node".into(),
                                peer_count: 0,
                                listen_addr: "127.0.0.1:4433".into(),
                            },
                            _ => RpcResponse::Error("not implemented".into()),
                        }
                    },
                    shutdown_rx,
                )
                .await;
        });

        // Give the server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let response = send_request(tmp.path(), &RpcRequest::Ping).await.unwrap();

        if let RpcResponse::Pong {
            node_id,
            peer_count,
            ..
        } = response
        {
            assert_eq!(node_id, "test-node");
            assert_eq!(peer_count, 0);
        } else {
            panic!("expected Pong, got {response:?}");
        }

        let _ = shutdown_tx.send(true);
        let _ = server_task.await;

        // Socket should be cleaned up
        assert!(!socket_path(tmp.path()).exists());
    }

    #[tokio::test]
    async fn daemon_is_running_check() {
        let tmp = tempfile::tempdir().unwrap();

        // No socket → not running
        assert!(!daemon_is_running(tmp.path()).await);

        // Start a server
        let server = RpcServer::bind(tmp.path()).unwrap();
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let _server_task = tokio::spawn(async move {
            server
                .serve(|_req| async { RpcResponse::Ok }, shutdown_rx)
                .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(daemon_is_running(tmp.path()).await);
    }

    #[tokio::test]
    async fn rpc_request_response_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let server = RpcServer::bind(tmp.path()).unwrap();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let server_task = tokio::spawn(async move {
            server
                .serve(
                    |req| async move {
                        match req {
                            RpcRequest::ListTesseras => RpcResponse::TesseraList(Vec::new()),
                            RpcRequest::NodeStatus => RpcResponse::Status {
                                node_id: "abc".into(),
                                peer_count: 5,
                                tessera_count: 10,
                                listen_addr: "0.0.0.0:4433".into(),
                            },
                            _ => RpcResponse::Error("unhandled".into()),
                        }
                    },
                    shutdown_rx,
                )
                .await;
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Test ListTesseras
        let resp = send_request(tmp.path(), &RpcRequest::ListTesseras)
            .await
            .unwrap();
        if let RpcResponse::TesseraList(list) = resp {
            assert!(list.is_empty());
        } else {
            panic!("expected TesseraList");
        }

        // Test NodeStatus
        let resp = send_request(tmp.path(), &RpcRequest::NodeStatus)
            .await
            .unwrap();
        if let RpcResponse::Status {
            peer_count,
            tessera_count,
            ..
        } = resp
        {
            assert_eq!(peer_count, 5);
            assert_eq!(tessera_count, 10);
        } else {
            panic!("expected Status");
        }

        let _ = shutdown_tx.send(true);
        let _ = server_task.await;
    }
}
