#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use crate::error::RpcError;
use crate::frame::{read_frame, write_frame};
use crate::protocol::{Request, Response};

/// Maximum number of retries for transient errors.
const MAX_RETRIES: u32 = 1;
/// Delay between retries.
const RETRY_DELAY: Duration = Duration::from_millis(500);

#[cfg(unix)]
type Stream = UnixStream;

#[cfg(windows)]
type Stream = std::net::TcpStream;

/// Synchronous RPC client. Connects to the daemon socket (Unix) or
/// named pipe (Windows), sends one request, reads one response.
#[derive(Debug)]
pub struct DaemonClient {
    socket_path: std::path::PathBuf,
    stream: Stream,
}

impl DaemonClient {
    /// Connect to the daemon socket with timeouts.
    pub fn connect(socket_path: &Path) -> Result<Self, RpcError> {
        let stream = Self::open_stream(socket_path)?;
        Ok(Self {
            socket_path: socket_path.to_path_buf(),
            stream,
        })
    }

    #[cfg(unix)]
    fn open_stream(socket_path: &Path) -> Result<Stream, RpcError> {
        let stream = UnixStream::connect(socket_path).map_err(|source| {
            RpcError::ConnectionFailed {
                path: socket_path.to_path_buf(),
                source,
            }
        })?;
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        Ok(stream)
    }

    #[cfg(windows)]
    fn open_stream(socket_path: &Path) -> Result<Stream, RpcError> {
        // On Windows, connect via TCP loopback to the daemon's named pipe bridge.
        let stream = std::net::TcpStream::connect("127.0.0.1:17283").map_err(|source| {
            RpcError::ConnectionFailed {
                path: socket_path.to_path_buf(),
                source,
            }
        })?;
        stream.set_read_timeout(Some(Duration::from_secs(120)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;
        Ok(stream)
    }

    /// Send a request and read the response.
    ///
    /// Automatically retries once on transient I/O errors (broken pipe,
    /// connection reset) by reconnecting to the daemon socket.
    pub fn call(&mut self, request: &Request) -> Result<Response, RpcError> {
        let mut last_err = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                std::thread::sleep(RETRY_DELAY);
                self.stream = Self::open_stream(&self.socket_path)?;
            }

            match self.try_call(request) {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_transient() && attempt < MAX_RETRIES => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_err.unwrap())
    }

    fn try_call(&mut self, request: &Request) -> Result<Response, RpcError> {
        write_frame(&mut self.stream, request)?;
        let response: Response = read_frame(&mut self.stream)?;

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
