use std::time::Duration;

pub struct ReplicationConfig {
    /// How often the repair loop runs. Default: 24h.
    pub repair_interval: Duration,
    /// Random jitter added to repair interval. Default: 0-2h.
    pub repair_jitter: Duration,
    /// Max concurrent fragment transfers. Default: 4.
    pub max_concurrent_transfers: usize,
    /// Minimum free disk space to maintain. Default: 1 GB.
    pub min_free_space_bytes: u64,
    /// Accept fragments even in reciprocity deficit, up to this amount. Default: 256 MB.
    pub accept_deficit_up_to_bytes: u64,
    /// Maximum total storage used for any single peer's fragments. Default: 1 GB.
    pub max_storage_per_peer_bytes: u64,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            repair_interval: Duration::from_secs(24 * 60 * 60),
            repair_jitter: Duration::from_secs(2 * 60 * 60),
            max_concurrent_transfers: 4,
            min_free_space_bytes: 1024 * 1024 * 1024, // 1 GB
            accept_deficit_up_to_bytes: 256 * 1024 * 1024, // 256 MB
            max_storage_per_peer_bytes: 1024 * 1024 * 1024, // 1 GB
        }
    }
}
