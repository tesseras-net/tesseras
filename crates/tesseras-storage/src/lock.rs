use std::fs::File;
use std::path::{Path, PathBuf};

use fs2::FileExt;

use crate::error::StorageError;

/// Holds an exclusive lock on a data directory.
/// The lock is released when this guard is dropped.
#[derive(Debug)]
pub struct StorageLock {
    _file: File,
    path: PathBuf,
}

impl StorageLock {
    /// Acquire an exclusive lock on `{data_dir}/tesseras.lock`.
    ///
    /// Returns an error if another process already holds the lock.
    /// The CLI does NOT use this — only long-lived processes (daemon, embedded node).
    pub fn acquire(data_dir: &Path) -> Result<Self, StorageError> {
        std::fs::create_dir_all(data_dir).map_err(StorageError::Io)?;

        let lock_path = data_dir.join("tesseras.lock");
        let file = File::create(&lock_path).map_err(StorageError::Io)?;

        file.try_lock_exclusive().map_err(|_| {
            StorageError::Database(format!(
                "another tesseras process is using {}. \
                 Stop it before starting a new one \
                 (e.g. `systemctl --user stop tesd`)",
                data_dir.display()
            ))
        })?;

        Ok(Self {
            _file: file,
            path: lock_path,
        })
    }

    /// Path to the lock file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_lock_succeeds() {
        let dir = TempDir::new().unwrap();
        let lock = StorageLock::acquire(dir.path());
        assert!(lock.is_ok());
    }

    #[test]
    fn second_lock_on_same_dir_fails() {
        let dir = TempDir::new().unwrap();
        let _lock1 = StorageLock::acquire(dir.path()).unwrap();
        let lock2 = StorageLock::acquire(dir.path());
        assert!(lock2.is_err());
        let err = lock2.unwrap_err().to_string();
        assert!(err.contains("another tesseras process"));
    }

    #[test]
    fn lock_released_on_drop() {
        let dir = TempDir::new().unwrap();
        {
            let _lock = StorageLock::acquire(dir.path()).unwrap();
        }
        // After drop, a new lock should succeed
        let lock2 = StorageLock::acquire(dir.path());
        assert!(lock2.is_ok());
    }
}
