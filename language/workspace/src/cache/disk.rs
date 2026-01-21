use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use filetime::{FileTime, set_file_mtime};
use fs2::FileExt;

use super::{CacheLock, CacheMetadata, CacheStore, CacheStoreError, CacheStoreKind};

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
    fn kind(&self) -> CacheStoreKind {
        CacheStoreKind::Disk
    }

    fn lock_shared(&self, path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
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
        FileExt::lock_shared(&file)?;

        Ok(CacheLock::File(file))
    }

    fn lock_exclusive(&self, path: &Path) -> Result<CacheLock<'_>, CacheStoreError> {
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

        Ok(CacheLock::File(file))
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, CacheStoreError> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }

    fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), CacheStoreError> {
        // ensure the parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // write to a temp path for atomic replace
        let temp_path = temp_path_for(path);
        let mut file = fs::File::create(&temp_path)?;
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;

        // close the temp file handle before rename
        drop(file);

        // rename into place with fallback for existing targets
        if let Err(error) = fs::rename(&temp_path, path) {
            if path.exists() {
                if let Err(remove_error) = fs::remove_file(path)
                    && remove_error.kind() != std::io::ErrorKind::NotFound
                {
                    cleanup_temp_path(&temp_path);
                    return Err(CacheStoreError::Io(remove_error));
                }
                if let Err(error) = fs::rename(&temp_path, path) {
                    cleanup_temp_path(&temp_path);
                    return Err(CacheStoreError::Io(error));
                }
            } else {
                cleanup_temp_path(&temp_path);
                return Err(CacheStoreError::Io(error));
            }
        }

        // best effort directory sync
        sync_parent_dir(path);

        Ok(())
    }

    fn touch(&self, path: &Path) -> Result<(), CacheStoreError> {
        let now = FileTime::from_system_time(SystemTime::now());
        match set_file_mtime(path, now) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }

    fn exists(&self, path: &Path) -> Result<bool, CacheStoreError> {
        Ok(path.exists())
    }

    fn remove(&self, path: &Path) -> Result<(), CacheStoreError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }

    fn metadata(&self, path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CacheStoreError::Io(error)),
        };

        let modified_ns = metadata.modified().ok().and_then(system_time_to_nanos);
        Ok(Some(CacheMetadata {
            size_bytes: metadata.len(),
            modified_ns,
        }))
    }
}

/// Convert a system time into nanoseconds since unix epoch.
fn system_time_to_nanos(time: SystemTime) -> Option<u64> {
    // compute nanoseconds since unix epoch
    let duration = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = duration.as_secs();
    let nanos = duration.subsec_nanos() as u64;
    seconds
        .checked_mul(1_000_000_000)
        .and_then(|base| base.checked_add(nanos))
}

/// Build a temp path for atomic replace.
fn temp_path_for(path: &Path) -> PathBuf {
    // resolve target directory and file name
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cache");

    // build a temp path for atomic replace
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temp_name = format!("{file_name}.tmp-{}-{timestamp}", std::process::id());
    dir.join(temp_name)
}

/// Sync the parent directory to persist rename metadata.
fn sync_parent_dir(path: &Path) {
    // sync the parent directory to persist rename metadata
    if let Some(parent) = path.parent()
        && let Ok(dir_file) = fs::File::open(parent)
    {
        let _ = dir_file.sync_all();
    }
}

/// Remove a temp cache file without failing the caller.
fn cleanup_temp_path(path: &Path) {
    // ignore cleanup failures
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::DiskCacheStore;
    use crate::CacheStore;
    use destack_source::TemporaryPhysicalFileSystem;

    /// Roundtrip cache payloads through the disk store.
    #[test]
    fn test_disk_cache_store_roundtrip() {
        let root = TemporaryPhysicalFileSystem::new_with_prefix("cache_store");
        let store = DiskCacheStore::new();
        let path = root.path_for(Path::new("nested/cache.bin"));
        let lock_path = root.path_for(Path::new("cache.lock"));

        let _lock = store.lock_exclusive(&lock_path).unwrap();
        store.write_atomic(&path, b"hello").unwrap();

        let bytes = store.read(&path).unwrap().unwrap();
        assert_eq!(bytes, b"hello");

        let metadata = store.metadata(&path).unwrap().unwrap();
        assert_eq!(metadata.size_bytes, 5);
        assert!(store.exists(&path).unwrap());

        store.remove(&path).unwrap();
        assert!(!store.exists(&path).unwrap());
        assert!(store.read(&path).unwrap().is_none());
    }

    /// Touching a missing cache file is a no op.
    #[test]
    fn test_disk_cache_store_touch_missing() {
        let root = TemporaryPhysicalFileSystem::new_with_prefix("cache_touch");

        let store = DiskCacheStore::new();
        let path = root.path_for(Path::new("missing.bin"));

        store.touch(&path).unwrap();
    }
}
