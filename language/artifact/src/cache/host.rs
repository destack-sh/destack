use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use filetime::{FileTime, set_file_mtime};
use fs2::FileExt;

use crate::cache::{CacheLock, CacheMetadata, CacheStore, CacheStoreError, CacheStoreKind};

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

    fn read_prefix(&self, path: &Path, limit: usize) -> Result<Option<Vec<u8>>, CacheStoreError> {
        let mut file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(CacheStoreError::Io(error)),
        };
        let mut bytes = vec![0; limit];
        let read_bytes = std::io::Read::read(&mut file, &mut bytes)?;
        bytes.truncate(read_bytes);

        Ok(Some(bytes))
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
        if let Err(error) = fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(CacheStoreError::Io(error));
        }

        Ok(())
    }

    fn metadata(&self, path: &Path) -> Result<Option<CacheMetadata>, CacheStoreError> {
        match fs::metadata(path) {
            Ok(metadata) => {
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                    .map(|value| value.as_nanos() as u64);
                Ok(Some(CacheMetadata {
                    size_bytes: metadata.len(),
                    modified_ns: modified,
                }))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(CacheStoreError::Io(error)),
        }
    }
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = match path.file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => String::from("cache"),
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let temp_name = format!("{file_name}.{timestamp}.tmp");
    path.with_file_name(temp_name)
}

fn cleanup_temp_path(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!("failed to clean temp cache file: {error}");
    }
}

fn sync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent()
        && let Ok(directory) = fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
}
