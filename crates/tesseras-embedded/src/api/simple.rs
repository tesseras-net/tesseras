//! FRB-exposed API functions wrapping EmbeddedNode as a global singleton.
//! flutter_rust_bridge generates Dart bindings from these public functions.

use std::sync::Mutex;

use crate::node::EmbeddedNode;
use crate::types::{IdentityInfo, MemoryInfo, NetworkStats, ReplicationStatus};

static NODE: Mutex<Option<EmbeddedNode>> = Mutex::new(None);

fn with_node<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&EmbeddedNode) -> Result<R, String>,
{
    let guard = NODE
        .lock()
        .map_err(|e| format!("node lock poisoned: {e}"))?;
    let node = guard
        .as_ref()
        .ok_or_else(|| "node not started".to_string())?;
    f(node)
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Start the embedded node with the given data directory.
#[flutter_rust_bridge::frb(sync)]
pub fn node_start(data_dir: String) -> Result<(), String> {
    let mut guard = NODE.lock().map_err(|e| e.to_string())?;
    if guard.is_some() {
        return Err("Node already running".to_string());
    }
    let node = EmbeddedNode::start(data_dir).map_err(|e| e.to_string())?;
    *guard = Some(node);
    Ok(())
}

/// Stop the embedded node.
#[flutter_rust_bridge::frb(sync)]
pub fn node_stop() -> Result<(), String> {
    let mut guard = NODE.lock().map_err(|e| e.to_string())?;
    let node = guard.take().ok_or("Node not running")?;
    node.stop().map_err(|e| e.to_string())
}

/// Check if the node is currently running.
#[flutter_rust_bridge::frb(sync)]
pub fn node_is_running() -> bool {
    NODE.lock()
        .ok()
        .and_then(|g| g.as_ref().map(|n| n.is_running()))
        .unwrap_or(false)
}

/// Create a user identity (profile).
#[flutter_rust_bridge::frb(sync)]
pub fn create_identity(name: String, avatar_path: Option<String>) -> Result<IdentityInfo, String> {
    with_node(|node| {
        node.create_identity(name.clone(), avatar_path.clone())
            .map_err(|e| e.to_string())
    })
}

/// Get the current user identity if one exists.
#[flutter_rust_bridge::frb(sync)]
pub fn get_identity() -> Result<Option<IdentityInfo>, String> {
    with_node(|node| node.get_identity().map_err(|e| e.to_string()))
}

/// Create a new memory (add tessera from a media file).
#[flutter_rust_bridge::frb(sync)]
pub fn create_memory(
    media_path: String,
    name: Option<String>,
    visibility: String,
) -> Result<MemoryInfo, String> {
    with_node(|node| {
        node.create_memory(media_path.clone(), name.clone(), visibility.clone())
            .map_err(|e| e.to_string())
    })
}

/// Get timeline of memories with pagination.
#[flutter_rust_bridge::frb(sync)]
pub fn get_timeline(offset: u32, limit: u32) -> Result<Vec<MemoryInfo>, String> {
    with_node(|node| node.get_timeline(offset, limit).map_err(|e| e.to_string()))
}

/// Get a single memory by its content hash.
#[flutter_rust_bridge::frb(sync)]
pub fn get_memory(hash: String) -> Result<Option<MemoryInfo>, String> {
    with_node(|node| node.get_memory(hash.clone()).map_err(|e| e.to_string()))
}

/// Get current network statistics.
#[flutter_rust_bridge::frb(sync)]
pub fn get_network_stats() -> Result<NetworkStats, String> {
    with_node(|node| node.get_network_stats().map_err(|e| e.to_string()))
}

/// Get current replication status.
#[flutter_rust_bridge::frb(sync)]
pub fn get_replication_status() -> Result<ReplicationStatus, String> {
    with_node(|node| node.get_replication_status().map_err(|e| e.to_string()))
}
