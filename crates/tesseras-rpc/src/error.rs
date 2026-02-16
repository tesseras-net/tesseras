use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("cannot connect to daemon at {path}: {source}")]
    ConnectionFailed {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("cannot determine socket path — set --socket or $XDG_RUNTIME_DIR")]
    NoSocketPath,

    #[error("daemon error: [{code:?}] {message}")]
    DaemonError {
        code: ErrorCode,
        message: String,
    },

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ErrorCode {
    NotFound,
    DaemonBusy,
    NetworkUnavailable,
    Internal,
}
