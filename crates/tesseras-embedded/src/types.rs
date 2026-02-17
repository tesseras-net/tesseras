use serde::{Deserialize, Serialize};

/// User identity / profile information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub name: String,
    pub avatar_path: Option<String>,
    pub public_key_hex: String,
    pub node_id_hex: String,
    pub created_at: String,
}

/// A single memory as returned to the Flutter UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub hash: String,
    pub tessera_hash: String,
    pub filename: String,
    pub media_type: String,
    pub size: u64,
    pub visibility: String,
    pub created_at: String,
    pub name: Option<String>,
}

/// Network statistics.
#[derive(Debug, Clone, Default)]
pub struct NetworkStats {
    pub peer_count: u32,
    pub is_bootstrapped: bool,
    pub node_id_hex: String,
    pub listen_addr: String,
}

/// Replication health status.
#[derive(Debug, Clone, Default)]
pub struct ReplicationStatus {
    pub total_fragments: u32,
    pub healthy_fragments: u32,
    pub missing_fragments: u32,
    pub data_shards: u32,
    pub parity_shards: u32,
}
