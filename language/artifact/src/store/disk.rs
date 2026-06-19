use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use fs2::FileExt;

use super::{BlobStore, BlobStoreError};

/// Process-local counter used to allocate temporary blob paths.
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Blob store backed by disk.
#[derive(Debug, Default, Clone)]
pub struct DiskBlobStore;

impl DiskBlobStore {
    /// Create a disk blob store.
    pub fn new() -> Self {
        Self
    }
}

impl BlobStore for DiskBlobStore {
    fn with_exclusive_lock(
        &self,
        path: &Path,
        operation: &mut dyn FnMut(),
    ) -> Result<(), BlobStoreError> {
        // ensure the lock directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // open and lock the store lock file
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

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError> {
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), BlobStoreError> {
        // ensure the parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // write a complete sibling file before publishing
        let (temp_path, mut file) = create_temp_file(path)?;
        std::io::Write::write_all(&mut file, bytes)?;

        // close the temp file handle before publish
        drop(file);

        // publish without replacing an existing exact file
        match fs::hard_link(&temp_path, path) {
            Ok(()) => {
                cleanup_temp_path(&temp_path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_temp_path(&temp_path);
                return Err(BlobStoreError::AlreadyExists);
            }
            Err(error) => {
                cleanup_temp_path(&temp_path);
                return Err(BlobStoreError::from(error));
            }
        }

        Ok(())
    }

    fn byte_len(&self, path: &Path) -> Result<Option<u64>, BlobStoreError> {
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }

    fn entries(&self, root: &Path) -> Result<Vec<PathBuf>, BlobStoreError> {
        let mut paths = Vec::new();

        collect_entries(root, &mut paths)?;

        Ok(paths)
    }

    fn remove(&self, path: &Path) -> Result<(), BlobStoreError> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }
}

/// Collect blob files below one root.
fn collect_entries(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), BlobStoreError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BlobStoreError::from(error)),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_entries(&path, paths)?;
        } else if file_type.is_file() {
            paths.push(path);
        }
    }

    Ok(())
}

/// Create one temporary sibling file for atomic publishing.
fn create_temp_file(path: &Path) -> Result<(PathBuf, fs::File), BlobStoreError> {
    let file_name = match path.file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => {
            let error = std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("cannot build temp path for blob path without file name: {path:?}"),
            );

            return Err(BlobStoreError::from(error));
        }
    };

    loop {
        let process = std::process::id();
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{file_name}.{process}.{counter}.tmp");
        let temp_path = path.with_file_name(temp_name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path);

        match file {
            Ok(file) => return Ok((temp_path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(BlobStoreError::from(error)),
        }
    }
}

/// Remove one temporary file.
fn cleanup_temp_path(path: &Path) {
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!("failed to clean temp blob file: {error}");
    }
}
