use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Node configuration, loaded from config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Address to listen on for QUIC connections.
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,

    /// Bootstrap node addresses.
    #[serde(default)]
    pub bootstrap: Vec<String>,

    /// Reed-Solomon erasure coding: data shards.
    #[serde(default = "default_data_shards")]
    pub data_shards: usize,

    /// Reed-Solomon erasure coding: parity shards.
    #[serde(default = "default_parity_shards")]
    pub parity_shards: usize,

    /// STUN servers for external address discovery.
    #[serde(default = "default_stun_servers")]
    pub stun_servers: Vec<String>,

    /// Maximum bytes for storing other nodes' fragments (0 = unlimited).
    #[serde(default)]
    pub max_foreign_storage_bytes: u64,

    /// Maximum total storage bytes including own data (0 = unlimited).
    #[serde(default)]
    pub max_total_storage_bytes: u64,
}

fn default_listen() -> SocketAddr {
    "0.0.0.0:4433".parse().unwrap()
}

fn default_data_shards() -> usize {
    3
}

fn default_parity_shards() -> usize {
    2
}

fn default_stun_servers() -> Vec<String> {
    vec![
        "stun.l.google.com:19302".into(),
        "stun1.l.google.com:19302".into(),
    ]
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen: default_listen(),
            bootstrap: Vec::new(),
            data_shards: default_data_shards(),
            parity_shards: default_parity_shards(),
            stun_servers: default_stun_servers(),
            max_foreign_storage_bytes: 0,
            max_total_storage_bytes: 0,
        }
    }
}

/// Data directory layout under ~/.tesseras/ (or custom path).
#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// Open or create the data directory at the given path.
    pub fn open(root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("blobs"))?;
        Ok(Self { root })
    }

    /// Default data directory: ~/.tesseras/
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .expect("no home directory")
            .join(".tesseras")
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn identity_key_path(&self) -> PathBuf {
        self.root.join("identity.key")
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn database_path(&self) -> PathBuf {
        self.root.join("tesseras.db")
    }

    pub fn blobs_dir(&self) -> PathBuf {
        self.root.join("blobs")
    }

    /// Path to a blob file, using 2-level directory sharding: ab/cd/abcdef...
    pub fn blob_path(&self, hash_hex: &str) -> PathBuf {
        let dir1 = &hash_hex[..2];
        let dir2 = &hash_hex[2..4];
        self.root.join("blobs").join(dir1).join(dir2).join(hash_hex)
    }

    /// Load config from config.toml, or return default if missing.
    pub fn load_config(&self) -> Result<NodeConfig, Box<dyn std::error::Error>> {
        let path = self.config_path();
        if path.exists() {
            let contents = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&contents)?)
        } else {
            Ok(NodeConfig::default())
        }
    }

    /// Save config to config.toml.
    pub fn save_config(&self, config: &NodeConfig) -> Result<(), Box<dyn std::error::Error>> {
        let contents = toml::to_string_pretty(config)?;
        std::fs::write(self.config_path(), contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_dir_creates_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = DataDir::open(tmp.path().join("tesseras")).unwrap();

        assert!(dir.root().exists());
        assert!(dir.blobs_dir().exists());
    }

    #[test]
    fn blob_path_sharding() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = DataDir::open(tmp.path()).unwrap();

        let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let path = dir.blob_path(hash);
        assert!(path.to_str().unwrap().contains("ab/cd/"));
    }

    #[test]
    fn config_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = DataDir::open(tmp.path()).unwrap();

        let mut config = NodeConfig::default();
        config.bootstrap.push("1.2.3.4:4433".into());
        dir.save_config(&config).unwrap();

        let loaded = dir.load_config().unwrap();
        assert_eq!(loaded.bootstrap, vec!["1.2.3.4:4433"]);
    }

    #[test]
    fn config_default_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = DataDir::open(tmp.path()).unwrap();

        let config = dir.load_config().unwrap();
        assert_eq!(config.listen.port(), 4433);
        assert!(config.bootstrap.is_empty());
    }
}
