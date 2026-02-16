pub mod handler;
pub mod import;
pub mod pack;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::watch;
use tracing::{error, info, warn};

use handler::RpcHandler;

/// Run the RPC listener. Accepts connections, dispatches to handler.
pub async fn run_listener(
    socket_path: PathBuf,
    handler: Arc<RpcHandler>,
    mut shutdown: watch::Receiver<bool>,
) {
    // Create parent directory with 0750 permissions (group-readable for CLI access)
    if let Some(parent) = socket_path.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            error!(path = %parent.display(), error = %e, "failed to create socket directory");
            return;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o750);
            if let Err(e) = std::fs::set_permissions(parent, perms) {
                warn!(path = %parent.display(), error = %e, "failed to set socket dir permissions");
            }
        }
    }

    // Stale socket cleanup
    if socket_path.exists() {
        match std::os::unix::net::UnixStream::connect(&socket_path) {
            Ok(_) => {
                error!(
                    path = %socket_path.display(),
                    "another daemon is already listening on this socket"
                );
                return;
            }
            Err(_) => {
                info!(path = %socket_path.display(), "removing stale socket");
                let _ = std::fs::remove_file(&socket_path);
            }
        }
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(l) => l,
        Err(e) => {
            error!(path = %socket_path.display(), error = %e, "failed to bind RPC socket");
            return;
        }
    };

    // Allow group members to connect (unix sockets require write permission)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o660);
        if let Err(e) = std::fs::set_permissions(&socket_path, perms) {
            warn!(path = %socket_path.display(), error = %e, "failed to set socket permissions");
        }
    }

    info!(path = %socket_path.display(), "RPC listener started");

    loop {
        tokio::select! {
            accept = listener.accept() => {
                match accept {
                    Ok((stream, _addr)) => {
                        let handler = Arc::clone(&handler);
                        tokio::spawn(async move {
                            if let Err(e) = handle_connection(stream, &handler).await {
                                warn!(error = %e, "RPC connection error");
                            }
                        });
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to accept RPC connection");
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    info!("RPC listener shutting down");
                    break;
                }
            }
        }
    }

    // Cleanup socket file
    let _ = std::fs::remove_file(&socket_path);
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    handler: &RpcHandler,
) -> Result<(), tesseras_rpc::RpcError> {
    // Convert tokio UnixStream to std for sync frame read/write
    let std_stream = stream.into_std()?;
    let mut reader = std::io::BufReader::new(std_stream.try_clone()?);
    let mut writer = std_stream;

    let request: tesseras_rpc::Request = tesseras_rpc::read_frame(&mut reader)?;
    let response = handler.handle(request).await;
    tesseras_rpc::write_frame(&mut writer, &response)?;
    Ok(())
}

/// Resolve the socket path from config or defaults.
pub fn resolve_socket_path(config_path: &Option<String>) -> Result<PathBuf, anyhow::Error> {
    if let Some(p) = config_path {
        Ok(PathBuf::from(p))
    } else {
        tesseras_rpc::default_socket_path().map_err(|e| anyhow::anyhow!("{e}"))
    }
}
