//! Flat FFI-friendly types for the embedded node boundary.
//! No trait objects, no generics, no Box<dyn> crossing FFI.

use tesseras_core::enums::{MemoryType, Visibility};

/// User identity information (flat for FFI).
#[derive(Debug, Clone)]
pub struct IdentityInfo {
    pub name: String,
    pub avatar_path: Option<String>,
    pub public_key_hex: String,
    pub node_id_hex: String,
    pub created_at: String, // ISO 8601
}

/// Request to create a new memory.
#[derive(Debug, Clone)]
pub struct CreateMemoryRequest {
    pub media_path: String,
    pub context_text: Option<String>,
    pub memory_type: MemoryType,
    pub visibility: Visibility,
    pub location_description: Option<String>,
    pub location_lat: Option<f64>,
    pub location_lon: Option<f64>,
    pub tags: Vec<String>,
    pub people: Vec<String>,
}

/// Memory information returned to Flutter (flat for FFI).
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub hash: String,
    pub tessera_hash: String,
    pub media_path: String,
    pub context: Option<String>,
    pub memory_type: String,
    pub visibility: String,
    pub created_at: String, // ISO 8601
    pub tags: Vec<String>,
}

/// Network statistics (flat for FFI).
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub peer_count: u32,
    pub dht_size: u32,
    pub is_bootstrapped: bool,
    pub uptime_secs: u64,
}

/// Replication status (flat for FFI).
#[derive(Debug, Clone)]
pub struct ReplicationStatus {
    pub total_fragments: u32,
    pub healthy_fragments: u32,
    pub repairing_fragments: u32,
    pub replication_factor: u32,
}

/// Network event for the live stream.
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    PeerConnected {
        node_id: String,
        addr: String,
    },
    PeerDisconnected {
        node_id: String,
    },
    ReplicationProgress {
        tessera_hash: String,
        fragments_distributed: u32,
        total: u32,
    },
    BootstrapComplete,
    AttestationReceived {
        fragment_id: String,
        from_node: String,
    },
    RepairTriggered {
        fragment_id: String,
    },
}
