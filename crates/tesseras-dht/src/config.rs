use std::time::Duration;

use tesseras_core::Capabilities;

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
    /// How often to check if re-bootstrap is needed (when routing table is empty).
    pub re_bootstrap_interval: Duration,
    /// Capabilities advertised in Pong responses.
    pub capabilities: Capabilities,
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
            stale_check_interval: Duration::from_secs(60),
            re_bootstrap_interval: Duration::from_secs(30),
            capabilities: Capabilities::phase2_default(),
        }
    }
}
