use std::path::PathBuf;

use async_trait::async_trait;
use tesseras_core::ports::{IdentityStore, KeyAlgorithm, KeyMaterial};
use tesseras_core::CoreError;

pub struct FsIdentityStore {
    base_path: PathBuf,
}

impl FsIdentityStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn key_path(&self, algorithm: KeyAlgorithm) -> PathBuf {
        let name = match algorithm {
            KeyAlgorithm::Ed25519 => "node.ed25519",
            KeyAlgorithm::MlDsa => "node.mldsa",
        };
        self.base_path.join("identity").join(name)
    }
}

#[async_trait]
impl IdentityStore for FsIdentityStore {
    async fn save_keypair(&self, material: &KeyMaterial) -> Result<(), CoreError> {
        let base = self.key_path(material.algorithm);
        tokio::fs::create_dir_all(base.parent().unwrap())
            .await
            .map_err(CoreError::Io)?;
        let key_path = format!("{}.key", base.display());
        let pub_path = format!("{}.pub", base.display());
        tokio::fs::write(&key_path, &material.secret)
            .await
            .map_err(CoreError::Io)?;
        tokio::fs::write(&pub_path, &material.public)
            .await
            .map_err(CoreError::Io)?;
        Ok(())
    }

    async fn load_keypair(&self, algorithm: KeyAlgorithm) -> Result<KeyMaterial, CoreError> {
        let base = self.key_path(algorithm);
        let key_path = format!("{}.key", base.display());
        let pub_path = format!("{}.pub", base.display());
        let secret = tokio::fs::read(&key_path).await.map_err(CoreError::Io)?;
        let public = tokio::fs::read(&pub_path).await.map_err(CoreError::Io)?;
        Ok(KeyMaterial {
            algorithm,
            secret,
            public,
        })
    }

    async fn keypair_exists(&self, algorithm: KeyAlgorithm) -> Result<bool, CoreError> {
        let base = self.key_path(algorithm);
        let key_path = format!("{}.key", base.display());
        Ok(tokio::fs::try_exists(&key_path).await.unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        let material = KeyMaterial {
            algorithm: KeyAlgorithm::Ed25519,
            secret: vec![0x42; 32],
            public: vec![0x43; 32],
        };
        store.save_keypair(&material).await.unwrap();
        let loaded = store.load_keypair(KeyAlgorithm::Ed25519).await.unwrap();
        assert_eq!(loaded, material);
    }

    #[tokio::test]
    async fn keypair_exists_false_initially() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        assert!(!store.keypair_exists(KeyAlgorithm::Ed25519).await.unwrap());
    }

    #[tokio::test]
    async fn keypair_exists_true_after_save() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        let material = KeyMaterial {
            algorithm: KeyAlgorithm::Ed25519,
            secret: vec![0x42; 32],
            public: vec![0x43; 32],
        };
        store.save_keypair(&material).await.unwrap();
        assert!(store.keypair_exists(KeyAlgorithm::Ed25519).await.unwrap());
    }
}
