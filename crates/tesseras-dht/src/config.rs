use std::time::Duration;

#[derive(Debug, Clone)]
pub struct DhtConfig {
    pub k: usize,
    pub alpha: usize,
    pub rpc_timeout: Duration,
    pub bucket_refresh_interval: Duration,
    pub republish_interval: Duration,
    pub pointer_ttl: Duration,
    pub max_stored_pointers: usize,
    pub ping_failure_threshold: u32,
    pub stale_check_interval: Duration,
}

impl Default for DhtConfig {
    fn default() -> Self {
        Self {
            k: 20,
            alpha: 3,
            rpc_timeout: Duration::from_secs(5),
            bucket_refresh_interval: Duration::from_secs(3600),
            republish_interval: Duration::from_secs(3600),
            pointer_ttl: Duration::from_secs(86400),
            max_stored_pointers: 100_000,
            ping_failure_threshold: 3,
            stale_check_interval: Duration::from_secs(900),
        }
    }
}
