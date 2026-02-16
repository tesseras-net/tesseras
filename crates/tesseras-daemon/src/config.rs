use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DaemonConfig {
    pub node: NodeConfig,
    pub dht: DhtTomlConfig,
    pub bootstrap: BootstrapConfig,
    pub network: NetworkConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub replication: ReplicationTomlConfig,
    #[serde(default)]
    pub nat: NatConfig,
    #[serde(default)]
    pub performance: PerformanceConfig,
    #[serde(default)]
    pub institutional: Option<InstitutionalConfig>,
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub listen_addr: SocketAddr,
    /// Additional listen addresses for explicit dual-stack (e.g. IPv4 + IPv6).
    #[serde(default)]
    pub listen_addrs: Vec<SocketAddr>,
}

impl NodeConfig {
    /// Return effective listen addresses: `listen_addrs` if non-empty,
    /// otherwise the single `listen_addr`.
    pub fn effective_addrs(&self) -> Vec<SocketAddr> {
        if !self.listen_addrs.is_empty() {
            self.listen_addrs.clone()
        } else {
            vec![self.listen_addr]
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DhtTomlConfig {
    pub k: usize,
    pub alpha: usize,
    pub bucket_refresh_interval_secs: u64,
    pub republish_interval_secs: u64,
    pub pointer_ttl_secs: u64,
    pub max_stored_pointers: usize,
    pub ping_failure_threshold: u32,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapConfig {
    pub dns_domain: String,
    pub hardcoded: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct NetworkConfig {
    pub enable_mdns: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ObservabilityConfig {
    pub metrics_addr: SocketAddr,
    pub log_format: String,
}

#[derive(Debug, Deserialize)]
pub struct ReplicationTomlConfig {
    /// How often the repair loop runs, in seconds. Default: 86400 (24h).
    pub repair_interval_secs: u64,
    /// Random jitter added to repair interval, in seconds. Default: 7200 (2h).
    pub repair_jitter_secs: u64,
}

impl Default for ReplicationTomlConfig {
    fn default() -> Self {
        Self {
            repair_interval_secs: 86400,
            repair_jitter_secs: 7200,
        }
    }
}

/// NAT traversal configuration.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct NatConfig {
    /// STUN server addresses for NAT detection.
    #[serde(default = "default_stun_servers")]
    pub stun_servers: Vec<String>,
    /// Enable relay functionality (forward traffic for NATed peers).
    #[serde(default)]
    pub relay_enabled: bool,
    /// Maximum relay sessions to serve simultaneously.
    #[serde(default = "default_relay_max_sessions")]
    pub relay_max_sessions: u16,
    /// Bandwidth limit for reciprocal relay peers (KB/s).
    #[serde(default = "default_relay_reciprocal_kbps")]
    pub relay_reciprocal_kbps: u32,
    /// Bandwidth limit for non-reciprocal (bootstrap) relay peers (KB/s).
    #[serde(default = "default_relay_bootstrap_kbps")]
    pub relay_bootstrap_kbps: u32,
    /// Relay session idle timeout in seconds.
    #[serde(default = "default_relay_idle_timeout_secs")]
    pub relay_idle_timeout_secs: u64,
}

fn default_stun_servers() -> Vec<String> {
    vec![
        "stun.l.google.com:19302".to_string(),
        "stun.cloudflare.com:3478".to_string(),
    ]
}

fn default_relay_max_sessions() -> u16 {
    50
}
fn default_relay_reciprocal_kbps() -> u32 {
    256
}
fn default_relay_bootstrap_kbps() -> u32 {
    64
}
fn default_relay_idle_timeout_secs() -> u64 {
    60
}

impl Default for NatConfig {
    fn default() -> Self {
        Self {
            stun_servers: default_stun_servers(),
            relay_enabled: false,
            relay_max_sessions: default_relay_max_sessions(),
            relay_reciprocal_kbps: default_relay_reciprocal_kbps(),
            relay_bootstrap_kbps: default_relay_bootstrap_kbps(),
            relay_idle_timeout_secs: default_relay_idle_timeout_secs(),
        }
    }
}

/// Performance tuning for SQLite, fragment cache, and connection pool.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PerformanceConfig {
    pub sqlite_synchronous_full: bool,
    pub sqlite_cache_size_kb: u32,
    pub sqlite_busy_timeout_ms: u32,
    pub fragment_cache_size_mb: u32,
    pub pool_max_connections: usize,
    pub pool_idle_timeout_secs: u64,
    pub pool_reaper_interval_secs: u64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            sqlite_synchronous_full: false,
            sqlite_cache_size_kb: 64000,
            sqlite_busy_timeout_ms: 5000,
            fragment_cache_size_mb: 128,
            pool_max_connections: 256,
            pool_idle_timeout_secs: 300,
            pool_reaper_interval_secs: 30,
        }
    }
}

/// Institutional node configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct InstitutionalConfig {
    /// Domain to verify via DNS TXT record.
    pub domain: String,
    /// Storage pledge in bytes.
    pub pledge_bytes: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                data_dir: dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("tesseras"),
                listen_addr: "0.0.0.0:4433".parse().unwrap(),
                listen_addrs: vec![],
            },
            dht: DhtTomlConfig {
                k: 20,
                alpha: 3,
                bucket_refresh_interval_secs: 3600,
                republish_interval_secs: 3600,
                pointer_ttl_secs: 86400,
                max_stored_pointers: 100_000,
                ping_failure_threshold: 3,
            },
            bootstrap: BootstrapConfig {
                dns_domain: "_tesseras._udp.tesseras.net".into(),
                hardcoded: vec![
                    "boot1.tesseras.net:4433".into(),
                    "boot2.tesseras.net:4433".into(),
                ],
            },
            network: NetworkConfig { enable_mdns: true },
            observability: ObservabilityConfig {
                metrics_addr: "127.0.0.1:9190".parse().unwrap(),
                log_format: "json".into(),
            },
            replication: ReplicationTomlConfig::default(),
            nat: NatConfig::default(),
            performance: PerformanceConfig::default(),
            institutional: None,
        }
    }
}

impl DaemonConfig {
    pub fn to_dht_config(&self) -> tesseras_dht::config::DhtConfig {
        tesseras_dht::config::DhtConfig {
            k: self.dht.k,
            alpha: self.dht.alpha,
            rpc_timeout: std::time::Duration::from_secs(5),
            bucket_refresh_interval: std::time::Duration::from_secs(
                self.dht.bucket_refresh_interval_secs,
            ),
            republish_interval: std::time::Duration::from_secs(self.dht.republish_interval_secs),
            pointer_ttl: std::time::Duration::from_secs(self.dht.pointer_ttl_secs),
            max_stored_pointers: self.dht.max_stored_pointers,
            ping_failure_threshold: self.dht.ping_failure_threshold,
            stale_check_interval: std::time::Duration::from_secs(900),
            capabilities: if self.institutional.is_some() {
                tesseras_core::Capabilities::institutional_default()
            } else {
                tesseras_core::Capabilities::phase2_default()
            },
        }
    }

    pub fn to_storage_config(&self) -> tesseras_storage::StorageConfig {
        tesseras_storage::StorageConfig {
            sqlite_synchronous_full: self.performance.sqlite_synchronous_full,
            sqlite_cache_size_kb: self.performance.sqlite_cache_size_kb,
            sqlite_busy_timeout_ms: self.performance.sqlite_busy_timeout_ms,
            fragment_cache_size_mb: self.performance.fragment_cache_size_mb,
        }
    }

    pub fn to_pool_config(&self) -> tesseras_net::PoolConfig {
        tesseras_net::PoolConfig {
            max_connections: self.performance.pool_max_connections,
            idle_timeout: std::time::Duration::from_secs(self.performance.pool_idle_timeout_secs),
            reaper_interval: std::time::Duration::from_secs(
                self.performance.pool_reaper_interval_secs,
            ),
        }
    }

    pub fn to_replication_config(&self) -> tesseras_replication::ReplicationConfig {
        tesseras_replication::ReplicationConfig {
            repair_interval: std::time::Duration::from_secs(self.replication.repair_interval_secs),
            repair_jitter: std::time::Duration::from_secs(self.replication.repair_jitter_secs),
            ..tesseras_replication::ReplicationConfig::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses() {
        let config = DaemonConfig::default();
        assert_eq!(config.dht.k, 20);
        assert_eq!(config.node.listen_addr.port(), 4433);
        assert!(config.node.listen_addrs.is_empty());
        assert_eq!(config.replication.repair_interval_secs, 86400);
        assert_eq!(config.replication.repair_jitter_secs, 7200);
    }

    #[test]
    fn effective_addrs_uses_listen_addrs_when_set() {
        let mut config = DaemonConfig::default();
        config.node.listen_addrs = vec![
            "0.0.0.0:4433".parse().unwrap(),
            "[::]:4433".parse().unwrap(),
        ];
        let addrs = config.node.effective_addrs();
        assert_eq!(addrs.len(), 2);
        assert!(addrs[0].is_ipv4());
        assert!(addrs[1].is_ipv6());
    }

    #[test]
    fn effective_addrs_falls_back_to_listen_addr() {
        let config = DaemonConfig::default();
        let addrs = config.node.effective_addrs();
        assert_eq!(addrs.len(), 1);
        assert_eq!(addrs[0], config.node.listen_addr);
    }

    #[test]
    fn toml_with_listen_addrs() {
        let toml = r#"
[node]
data_dir = "/tmp/test"
listen_addr = "127.0.0.1:4433"
listen_addrs = ["0.0.0.0:4433", "[::]:4433"]

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "test"
hardcoded = []

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
"#;
        let config: DaemonConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.node.listen_addrs.len(), 2);
        let addrs = config.node.effective_addrs();
        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn toml_without_replication_uses_defaults() {
        let toml = r#"
[node]
data_dir = "/tmp/test"
listen_addr = "127.0.0.1:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "test"
hardcoded = []

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
"#;
        let config: DaemonConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.replication.repair_interval_secs, 86400);
        assert_eq!(config.replication.repair_jitter_secs, 7200);
    }

    #[test]
    fn nat_config_defaults() {
        let config: NatConfig = toml::from_str("").unwrap();
        assert_eq!(config.stun_servers.len(), 2);
        assert!(!config.relay_enabled);
        assert_eq!(config.relay_max_sessions, 50);
    }

    #[test]
    fn nat_config_custom() {
        let toml_str = r#"
            stun_servers = ["stun.example.com:3478"]
            relay_enabled = true
            relay_max_sessions = 100
        "#;
        let config: NatConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.stun_servers.len(), 1);
        assert!(config.relay_enabled);
        assert_eq!(config.relay_max_sessions, 100);
    }

    #[test]
    fn toml_without_institutional_section_defaults_to_none() {
        let toml = r#"
[node]
data_dir = "/tmp/test"
listen_addr = "127.0.0.1:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "test"
hardcoded = []

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"
"#;
        let config: DaemonConfig = toml::from_str(toml).unwrap();
        assert!(config.institutional.is_none());
    }

    #[test]
    fn toml_with_institutional_section_parses() {
        let toml = r#"
[node]
data_dir = "/tmp/test"
listen_addr = "127.0.0.1:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "test"
hardcoded = []

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"

[institutional]
domain = "archive.org"
pledge_bytes = 536870912000
"#;
        let config: DaemonConfig = toml::from_str(toml).unwrap();
        let inst = config.institutional.unwrap();
        assert_eq!(inst.domain, "archive.org");
        assert_eq!(inst.pledge_bytes, 536_870_912_000);
    }

    #[test]
    fn toml_with_replication_overrides() {
        let toml = r#"
[node]
data_dir = "/tmp/test"
listen_addr = "127.0.0.1:4433"

[dht]
k = 20
alpha = 3
bucket_refresh_interval_secs = 3600
republish_interval_secs = 3600
pointer_ttl_secs = 86400
max_stored_pointers = 100000
ping_failure_threshold = 3

[bootstrap]
dns_domain = "test"
hardcoded = []

[network]
enable_mdns = false

[observability]
metrics_addr = "127.0.0.1:9190"
log_format = "json"

[replication]
repair_interval_secs = 5
repair_jitter_secs = 1
"#;
        let config: DaemonConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.replication.repair_interval_secs, 5);
        assert_eq!(config.replication.repair_jitter_secs, 1);
    }
}
