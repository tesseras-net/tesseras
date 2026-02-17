//! FRB-exposed API functions wrapping EmbeddedNode as a global singleton.
//! flutter_rust_bridge generates Dart bindings from these public functions.

use std::sync::Mutex;

use crate::error::TesserasError;
use crate::node::EmbeddedNode;
use crate::types::{
    CreateMemoryRequest, IdentityInfo, MemoryInfo, NetworkStats, PeerInfo, ReplicationStatus,
    StorageStats,
};

static NODE: Mutex<Option<EmbeddedNode>> = Mutex::new(None);

fn with_node<F, R>(f: F) -> Result<R, TesserasError>
where
    F: FnOnce(&EmbeddedNode) -> Result<R, TesserasError>,
{
    let guard = NODE
        .lock()
        .map_err(|e| TesserasError::Storage(format!("node lock poisoned: {e}")))?;
    let node = guard.as_ref().ok_or(TesserasError::NotInitialized)?;
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
    with_node(|node| node.create_identity(name.clone(), avatar_path.clone()))
        .map_err(|e| e.to_string())
}

/// Get the current user identity if one exists.
#[flutter_rust_bridge::frb(sync)]
pub fn get_identity() -> Result<Option<IdentityInfo>, String> {
    with_node(|node| node.get_identity()).map_err(|e| e.to_string())
}

/// Create a new memory (photo/video/text).
#[allow(clippy::too_many_arguments)]
#[flutter_rust_bridge::frb(sync)]
pub fn create_memory(
    media_path: String,
    context_text: Option<String>,
    memory_type: String,
    visibility: String,
    location_description: Option<String>,
    location_lat: Option<f64>,
    location_lon: Option<f64>,
    tags: Vec<String>,
    people: Vec<String>,
    sealed_open_after: Option<String>,
    inactive_years: Option<u32>,
) -> Result<MemoryInfo, String> {
    let memory_type = parse_memory_type(&memory_type)?;
    let visibility = parse_visibility(&visibility, sealed_open_after, inactive_years)?;

    let request = CreateMemoryRequest {
        media_path,
        context_text,
        memory_type,
        visibility,
        location_description,
        location_lat,
        location_lon,
        tags,
        people,
    };

    with_node(|node| node.create_memory(request.clone())).map_err(|e| e.to_string())
}

/// Get timeline of memories with pagination.
#[flutter_rust_bridge::frb(sync)]
pub fn get_timeline(offset: u32, limit: u32) -> Result<Vec<MemoryInfo>, String> {
    with_node(|node| node.get_timeline(offset, limit)).map_err(|e| e.to_string())
}

/// Get a single memory by its content hash.
#[flutter_rust_bridge::frb(sync)]
pub fn get_memory(hash: String) -> Result<MemoryInfo, String> {
    with_node(|node| node.get_memory(hash.clone())).map_err(|e| e.to_string())
}

/// Get current network statistics.
#[flutter_rust_bridge::frb(sync)]
pub fn get_network_stats() -> Result<NetworkStats, String> {
    with_node(|node| node.get_network_stats()).map_err(|e| e.to_string())
}

/// Get current replication status.
#[flutter_rust_bridge::frb(sync)]
pub fn get_replication_status() -> Result<ReplicationStatus, String> {
    with_node(|node| node.get_replication_status()).map_err(|e| e.to_string())
}

/// Get list of connected peers.
#[flutter_rust_bridge::frb(sync)]
pub fn get_connected_peers() -> Result<Vec<PeerInfo>, String> {
    with_node(|node| node.get_connected_peers()).map_err(|e| e.to_string())
}

/// Get storage statistics.
#[flutter_rust_bridge::frb(sync)]
pub fn get_storage_stats() -> Result<StorageStats, String> {
    with_node(|node| node.get_storage_stats()).map_err(|e| e.to_string())
}

/// Read a media blob by tessera hash, memory hash, and file name.
#[flutter_rust_bridge::frb(sync)]
pub fn get_media_blob(
    tessera_hash: String,
    memory_hash: String,
    name: String,
) -> Result<Vec<u8>, String> {
    with_node(|node| node.get_media_blob(tessera_hash.clone(), memory_hash.clone(), name.clone()))
        .map_err(|e| e.to_string())
}

fn parse_memory_type(s: &str) -> Result<tesseras_core::enums::MemoryType, String> {
    match s.to_lowercase().as_str() {
        "moment" => Ok(tesseras_core::enums::MemoryType::Moment),
        "reflection" => Ok(tesseras_core::enums::MemoryType::Reflection),
        "daily" => Ok(tesseras_core::enums::MemoryType::Daily),
        "relation" => Ok(tesseras_core::enums::MemoryType::Relation),
        "object" => Ok(tesseras_core::enums::MemoryType::Object),
        _ => Err(format!("unknown memory type: {s}")),
    }
}

fn parse_visibility(
    s: &str,
    sealed_open_after: Option<String>,
    inactive_years: Option<u32>,
) -> Result<tesseras_core::enums::Visibility, String> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        "private" => Ok(tesseras_core::enums::Visibility::Private),
        "public" => Ok(tesseras_core::enums::Visibility::Public),
        "circle" => Ok(tesseras_core::enums::Visibility::Circle {
            circle: String::new(),
        }),
        _ if lower.starts_with("circle:") => Ok(tesseras_core::enums::Visibility::Circle {
            circle: lower.strip_prefix("circle:").unwrap().to_string(),
        }),
        "sealed" => {
            let open_after = sealed_open_after
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::days(365));
            Ok(tesseras_core::enums::Visibility::Sealed { open_after })
        }
        "publicafterdeath" => {
            let years = inactive_years.unwrap_or(5);
            Ok(tesseras_core::enums::Visibility::PublicAfterDeath {
                inactive_years: years,
            })
        }
        _ => Err(format!("unknown visibility: {s}")),
    }
}
