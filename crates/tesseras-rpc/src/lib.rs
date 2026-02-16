pub mod client;
pub mod error;
pub mod frame;
pub mod protocol;

use std::path::PathBuf;

pub use client::DaemonClient;
pub use error::{ErrorCode, RpcError};
pub use frame::{read_frame, write_frame};
pub use protocol::{PublishState, Request, Response};
pub use tesseras_core::NodeInfo;

/// Well-known socket path for system-level daemon (systemd RuntimeDirectory).
const SYSTEM_SOCKET: &str = "/run/tesseras/daemon.sock";

/// Default daemon socket path. Shared by client and server.
///
/// Resolution order:
/// 1. `$XDG_RUNTIME_DIR/tesseras/daemon.sock` (user session)
/// 2. `/run/tesseras/daemon.sock` (system service via RuntimeDirectory)
/// 3. `$XDG_DATA_HOME/tesseras/daemon.sock` (fallback)
pub fn default_socket_path() -> Result<PathBuf, RpcError> {
    if let Some(runtime) = dirs::runtime_dir() {
        let user_path = runtime.join("tesseras/daemon.sock");
        if user_path.exists() {
            return Ok(user_path);
        }
    }

    let system_path = PathBuf::from(SYSTEM_SOCKET);
    if system_path.exists() {
        return Ok(system_path);
    }

    // For server-side (creating the socket): prefer XDG_RUNTIME_DIR, then system path.
    if let Some(runtime) = dirs::runtime_dir() {
        Ok(runtime.join("tesseras/daemon.sock"))
    } else if let Some(data) = dirs::data_dir() {
        Ok(data.join("tesseras/daemon.sock"))
    } else {
        Err(RpcError::NoSocketPath)
    }
}
