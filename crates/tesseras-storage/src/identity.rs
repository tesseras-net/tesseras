use std::path::PathBuf;

use tesseras_core::CoreError;
use tesseras_core::ports::{IdentityStore, KeyAlgorithm, KeyMaterial};

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
            KeyAlgorithm::X25519 => "node.x25519",
            KeyAlgorithm::MlKem768 => "node.mlkem768",
        };
        self.base_path.join("identity").join(name)
    }
}

impl IdentityStore for FsIdentityStore {
    fn save_keypair(&self, material: &KeyMaterial) -> Result<(), CoreError> {
        let base = self.key_path(material.algorithm);
        std::fs::create_dir_all(base.parent().unwrap()).map_err(CoreError::Io)?;
        let key_path = format!("{}.key", base.display());
        let pub_path = format!("{}.pub", base.display());
        std::fs::write(&key_path, &material.secret).map_err(CoreError::Io)?;
        std::fs::write(&pub_path, &material.public).map_err(CoreError::Io)?;
        Ok(())
    }

    fn load_keypair(&self, algorithm: KeyAlgorithm) -> Result<KeyMaterial, CoreError> {
        let base = self.key_path(algorithm);
        let key_path = format!("{}.key", base.display());
        let pub_path = format!("{}.pub", base.display());
        let secret = std::fs::read(&key_path).map_err(CoreError::Io)?;
        let public = std::fs::read(&pub_path).map_err(CoreError::Io)?;
        Ok(KeyMaterial {
            algorithm,
            secret,
            public,
        })
    }

    fn keypair_exists(&self, algorithm: KeyAlgorithm) -> Result<bool, CoreError> {
        let base = self.key_path(algorithm);
        let key_path = format!("{}.key", base.display());
        Ok(std::path::Path::new(&key_path).exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        let material = KeyMaterial {
            algorithm: KeyAlgorithm::Ed25519,
            secret: vec![0x42; 32],
            public: vec![0x43; 32],
        };
        store.save_keypair(&material).unwrap();
        let loaded = store.load_keypair(KeyAlgorithm::Ed25519).unwrap();
        assert_eq!(loaded, material);
    }

    #[test]
    fn keypair_exists_false_initially() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        assert!(!store.keypair_exists(KeyAlgorithm::Ed25519).unwrap());
    }

    #[test]
    fn keypair_exists_true_after_save() {
        let dir = TempDir::new().unwrap();
        let store = FsIdentityStore::new(dir.path().to_path_buf());
        let material = KeyMaterial {
            algorithm: KeyAlgorithm::Ed25519,
            secret: vec![0x42; 32],
            public: vec![0x43; 32],
        };
        store.save_keypair(&material).unwrap();
        assert!(store.keypair_exists(KeyAlgorithm::Ed25519).unwrap());
    }
}
