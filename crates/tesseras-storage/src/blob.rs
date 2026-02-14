use std::path::PathBuf;

use async_trait::async_trait;
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

#[async_trait]
impl BlobStore for FsBlobStore {
    async fn write(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
        data: &[u8],
    ) -> Result<(), CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    async fn read(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<Vec<u8>, CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        tokio::fs::read(&path).await.map_err(CoreError::Io)
    }

    async fn exists(
        &self,
        tessera_hash: &ContentHash,
        memory_hash: &ContentHash,
        name: &str,
    ) -> Result<bool, CoreError> {
        let path = self.blob_path(tessera_hash, memory_hash, name);
        Ok(tokio::fs::try_exists(&path).await.unwrap_or(false))
    }

    async fn delete_tessera(&self, tessera_hash: &ContentHash) -> Result<(), CoreError> {
        let path = self.base_path.join(tessera_hash.to_string());
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_dir_all(&path).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn write_read_roundtrip() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        let data = b"JPEG image data here";
        store.write(&t_hash, &m_hash, "media.jpg", data).await.unwrap();
        let read = store.read(&t_hash, &m_hash, "media.jpg").await.unwrap();
        assert_eq!(read, data);
    }

    #[tokio::test]
    async fn read_nonexistent_returns_error() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        let result = store.read(&t_hash, &m_hash, "nope.jpg").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn exists_check() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").await.unwrap());
        store.write(&t_hash, &m_hash, "media.jpg", b"data").await.unwrap();
        assert!(store.exists(&t_hash, &m_hash, "media.jpg").await.unwrap());
    }

    #[tokio::test]
    async fn delete_tessera_removes_all() {
        let dir = TempDir::new().unwrap();
        let store = FsBlobStore::new(dir.path().join("blobs"));
        let t_hash = ContentHash::new([0x01; 32]);
        let m_hash = ContentHash::new([0x02; 32]);
        store.write(&t_hash, &m_hash, "media.jpg", b"data").await.unwrap();
        store.delete_tessera(&t_hash).await.unwrap();
        assert!(!store.exists(&t_hash, &m_hash, "media.jpg").await.unwrap());
    }
}
