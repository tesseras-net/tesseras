use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DaemonConfig {
    pub node: NodeConfig,
    pub dht: DhtTomlConfig,
    pub bootstrap: BootstrapConfig,
    pub network: NetworkConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Deserialize)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub listen_addr: SocketAddr,
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
pub struct NetworkConfig {
    pub enable_mdns: bool,
}

#[derive(Debug, Deserialize)]
pub struct ObservabilityConfig {
    pub metrics_addr: SocketAddr,
    pub log_format: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                data_dir: dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("tesseras"),
                listen_addr: "0.0.0.0:4433".parse().unwrap(),
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
    }
}
