use std::path::PathBuf;

use tesseras_core::ports::BlobStore;
use tesseras_core::{ContentHash, CoreError};

pub struct FsBlobStore {
    base_path: PathBuf,
}

impl FsBlobStore {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn blob_path(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> PathBuf {
        self.base_path
            .join(tessera_hash.to_string())
            .join(memory_hash.to_string())
            .join(name)
    }
}

impl BlobStore for FsBlobStore {
    fn write(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
        data: &[u8],
    ) -> Result<(), CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        std::fs::create_dir_all(path.parent().unwrap())?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    fn read(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<Vec<u8>, CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        std::fs::read(&path).map_err(CoreError::Io)
    }

    fn exists(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<bool, CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        Ok(path.exists())
    }

    fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError> {
        let path = self.base_path.join(tessera_hash.to_string());
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        let data = b"JPEG image data here";
        store.write(&t_hash, &m_hash, "media.jpg", data).unwrap();
        let read = store.read(&t_hash, &m_hash, "media.jpg").unwrap();
        assert_eq!(read, data);
    }

    #[test]
    fn read_nonexistent_returns_error() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        let result = store.read(&t_hash, &m_hash, "nope.jpg");
        assert!(result.is_err());
    }

    #[test]
    fn exists_check() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
        store.write(&t_hash, &m_hash, "media.jpg", b"data").unwrap();
        assert!(store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
    }

    #[test]
    fn delete_tessera_removes_all() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        store.write(&t_hash, &m_hash, "media.jpg", b"data").unwrap();
        store.delete_tessera(&t_hash).unwrap();
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").unwrap());
    }
}
