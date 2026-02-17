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
    DaemonError { code: ErrorCode, message: String },

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("{}", friendly_io_message(.0))]
    Io(#[from] std::io::Error),
}

impl RpcError {
    /// Whether this error is transient and worth retrying.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Io(e) => matches!(
                e.kind(),
                std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::WouldBlock
            ),
            _ => false,
        }
    }
}

fn friendly_io_message(err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset => {
            "daemon closed the connection — it may still be starting up, try again in a moment"
                .to_string()
        }
        std::io::ErrorKind::ConnectionAborted => {
            "connection to daemon was interrupted — try again".to_string()
        }
        std::io::ErrorKind::TimedOut => {
            "daemon did not respond in time — it may be overloaded or unresponsive".to_string()
        }
        _ => format!("io: {err}"),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ErrorCode {
    NotFound,
    NotOwner,
    DaemonBusy,
    NetworkUnavailable,
    CircleNotFound,
    ContactNotFound,
    AlreadyExists,
    InvalidInput,
    Internal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn broken_pipe_shows_friendly_message() {
        let err: RpcError = io::Error::from(io::ErrorKind::BrokenPipe).into();
        let msg = err.to_string();
        assert!(
            msg.contains("daemon closed the connection"),
            "expected friendly message, got: {msg}"
        );
        assert!(
            !msg.contains("os error"),
            "should not contain raw OS error: {msg}"
        );
    }

    #[test]
    fn connection_reset_shows_friendly_message() {
        let err: RpcError = io::Error::from(io::ErrorKind::ConnectionReset).into();
        let msg = err.to_string();
        assert!(msg.contains("daemon closed the connection"), "got: {msg}");
    }

    #[test]
    fn timed_out_shows_friendly_message() {
        let err: RpcError = io::Error::from(io::ErrorKind::TimedOut).into();
        let msg = err.to_string();
        assert!(msg.contains("did not respond in time"), "got: {msg}");
    }

    #[test]
    fn other_io_errors_preserve_original_message() {
        let err: RpcError = io::Error::from(io::ErrorKind::PermissionDenied).into();
        let msg = err.to_string();
        assert!(msg.starts_with("io:"), "got: {msg}");
    }

    #[test]
    fn broken_pipe_is_transient() {
        let err: RpcError = io::Error::from(io::ErrorKind::BrokenPipe).into();
        assert!(err.is_transient());
    }

    #[test]
    fn connection_reset_is_transient() {
        let err: RpcError = io::Error::from(io::ErrorKind::ConnectionReset).into();
        assert!(err.is_transient());
    }

    #[test]
    fn protocol_error_is_not_transient() {
        let err = RpcError::Protocol("bad frame".into());
        assert!(!err.is_transient());
    }

    #[test]
    fn daemon_error_is_not_transient() {
        let err = RpcError::DaemonError {
            code: ErrorCode::Internal,
            message: "boom".into(),
        };
        assert!(!err.is_transient());
    }
}
