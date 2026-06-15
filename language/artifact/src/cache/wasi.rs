use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::cache::{CacheStore, CacheStoreError};

/// Delay between cache lock acquisition attempts.
const CACHE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);

/// Cache store backed by WASI filesystem access.
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
        // acquire cache lock
        let _lock = WasiCacheLock::acquire(path)?;

        // run caller operation
        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        // read existing bytes
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::from(error)),
        }
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
        // ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // write temporary file
        let temp_path = temp_path_for(path);
        let mut file = fs::File::create(&temp_path)?;
        std::io::Write::write_all(&mut file, bytes)?;
        let _ = file.sync_all();

        // close file before publishing
        drop(file);

        // detect existing cache entry
        if path.exists() {
            cleanup_temp_path(&temp_path);

            return Err(CacheStoreError::AlreadyExists);
        }

        // publish temporary file
        match fs::rename(&temp_path, path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_temp_path(&temp_path);

                return Err(CacheStoreError::AlreadyExists);
            }
            Err(error) => {
                cleanup_temp_path(&temp_path);

                return Err(CacheStoreError::from(error));
            }
        }

        Ok(())
    }

    fn byte_len(&self, path: &Path) -> Result<Option<u64>, CacheStoreError> {
        // read file metadata
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::from(error)),
        }
    }

    fn entries(&self, root: &Path) -> Result<Vec<PathBuf>, CacheStoreError> {
        // collect files recursively
        let mut paths = Vec::new();
        collect_entries(root, &mut paths)?;

        Ok(paths)
    }

    fn remove(&self, path: &Path) -> Result<(), CacheStoreError> {
        // remove existing file
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(CacheStoreError::from(error)),
        }
    }
}

/// Held WASI cache lock file.
#[derive(Debug)]
struct WasiCacheLock {
    /// The lock file path.
    path: PathBuf,
}

impl WasiCacheLock {
    /// Acquire one cache lock file.
    fn acquire(path: &Path) -> Result<Self, CacheStoreError> {
        // ensure lock directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // create lock file
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(_file) => break,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(CACHE_LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(CacheStoreError::from(error)),
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for WasiCacheLock {
    fn drop(&mut self) {
        // release cache lock
        cleanup_temp_path(&self.path);
    }
}

/// Collect cache entry files below one root.
fn collect_entries(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), CacheStoreError> {
    // read root directory
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(CacheStoreError::from(error)),
    };

    // collect child entries
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        // recurse into directories
        if file_type.is_dir() {
            collect_entries(&path, paths)?;
        }
        // collect files
        else if file_type.is_file() {
            paths.push(path);
        }
    }

    Ok(())
}

/// Return one temp path for atomic publishing.
fn temp_path_for(path: &Path) -> PathBuf {
    // extract file name
    let file_name = match path.file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => panic!("cannot build temp path for cache path without file name: {path:?}"),
    };

    // build unique suffix
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time before unix epoch for cache write: {error}"))
        .as_nanos();
    let temp_name = format!("{file_name}.{timestamp}.tmp");

    path.with_file_name(temp_name)
}

/// Remove one temporary or lock file.
fn cleanup_temp_path(path: &Path) {
    // remove file if present
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!("failed to clean cache file: {error}");
    }
}
