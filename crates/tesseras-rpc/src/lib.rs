pub mod client;
pub mod error;
pub mod frame;
pub mod protocol;

use std::path::PathBuf;

pub use client::DaemonClient;
pub use error::{ErrorCode, RpcError};
pub use frame::{read_frame, write_frame};
pub use protocol::{PublishState, Request, Response};

/// Default daemon socket path. Shared by client and server.
pub fn default_socket_path() -> Result<PathBuf, RpcError> {
    if let Some(runtime) = dirs::runtime_dir() {
        Ok(runtime.join("tesseras/daemon.sock"))
    } else if let Some(data) = dirs::data_dir() {
        Ok(data.join("tesseras/daemon.sock"))
    } else {
        Err(RpcError::NoSocketPath)
    }
}
