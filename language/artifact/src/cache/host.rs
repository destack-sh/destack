use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use fs2::FileExt;

use crate::cache::{CacheStore, CacheStoreError};

/// Cache store backed by disk.
#[derive(Debug, Default, Clone)]
pub struct DiskCacheStore;

impl DiskCacheStore {
    /// Create a disk cache store.
    pub fn new() -> Self {
        Self
    }
}

impl CacheStore for DiskCacheStore {
    fn with_exclusive_lock(
        &self,
        path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), CacheStoreError> {
        // ensure the lock directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // open and lock the cache lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        FileExt::lock_exclusive(&file)?;
        let _lock = file;

        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
        // ensure the parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // write to a temp path before publishing
        let temp_path = temp_path_for(path);
        let mut file = fs::File::create(&temp_path)?;
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;

        // close the temp file handle before rename
        drop(file);

        // publish without replacing an existing exact image
        match fs::hard_link(&temp_path, path) {
            Ok(()) => {
                cleanup_temp_path(&temp_path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_temp_path(&temp_path);
                return Err(CacheStoreError::AlreadyExists);
            }
            Err(error) => {
                cleanup_temp_path(&temp_path);
                return Err(CacheStoreError::Io(error));
            }
        }

        // best effort directory sync
        sync_parent_dir(path);

        Ok(())
    }

    fn byte_len(&self, path: &Path) -> Result<Option<u64>, CacheStoreError> {
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }
}

/// Return one temp path for atomic replacement.
fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = match path.file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => panic!("cannot build temp path for cache path without file name: {path:?}"),
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time before unix epoch for cache write: {error}"))
        .as_nanos();

    let temp_name = format!("{file_name}.{timestamp}.tmp");
    path.with_file_name(temp_name)
}

/// Remove one temp path after a failed write.
fn cleanup_temp_path(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!("failed to clean temp cache file: {error}");
    }
}

/// Sync the parent directory after one atomic replace.
fn sync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent()
        && let Ok(directory) = fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
}
