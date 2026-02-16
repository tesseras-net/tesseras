use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::error::RpcError;
use crate::frame::{read_frame, write_frame};
use crate::protocol::{Request, Response};

/// Synchronous RPC client. Connects to the daemon Unix socket,
/// sends one request, reads one response, then the connection is dropped.
#[derive(Debug)]
pub struct DaemonClient {
    stream: UnixStream,
}

impl DaemonClient {
    /// Connect to the daemon socket with timeouts.
    pub fn connect(socket_path: &Path) -> Result<Self, RpcError> {
        let stream = UnixStream::connect(socket_path).map_err(|source| {
            RpcError::ConnectionFailed {
                path: socket_path.to_path_buf(),
                source,
            }
        })?;
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        Ok(Self { stream })
    }

    /// Send a request and read the response.
    pub fn call(&mut self, request: &Request) -> Result<Response, RpcError> {
        write_frame(&mut self.stream, request)?;
        let response: Response = read_frame(&mut self.stream)?;

        // Convert daemon-side errors into RpcError
        if let Response::Error { code, message } = &response {
            return Err(RpcError::DaemonError {
                code: code.clone(),
                message: message.clone(),
            });
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn connect_fails_with_clear_error_on_missing_socket() {
        let path = PathBuf::from("/tmp/tesseras-test-nonexistent.sock");
        let result = DaemonClient::connect(&path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            RpcError::ConnectionFailed { path: p, .. } => {
                assert_eq!(p, path);
            }
            other => panic!("expected ConnectionFailed, got: {other}"),
        }
    }
}
