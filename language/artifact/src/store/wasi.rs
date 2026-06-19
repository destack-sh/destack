use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use super::{BlobStore, BlobStoreError};

/// Delay between store lock acquisition attempts.
const STORE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);
/// Process-local counter used to allocate temporary blob paths.
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Blob store backed by WASI filesystem access.
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
        // acquire store lock
        let _lock = WasiStoreLock::acquire(path)?;

        // run caller operation
        operation();

        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BlobStoreError> {
        // read existing bytes
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }

    fn write_once(&self, path: &Path, bytes: &[u8]) -> Result<(), BlobStoreError> {
        // ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // write a complete sibling file before publishing
        let (temp_path, mut file) = create_temp_file(path)?;
        std::io::Write::write_all(&mut file, bytes)?;

        // close file before publishing
        drop(file);

        // detect existing blob
        if path.exists() {
            cleanup_temp_path(&temp_path);

            return Err(BlobStoreError::AlreadyExists);
        }

        // publish temporary file
        match fs::rename(&temp_path, path) {
            Ok(()) => {}
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
        // read file metadata
        match fs::metadata(path) {
            Ok(metadata) => Ok(Some(metadata.len())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }

    fn entries(&self, root: &Path) -> Result<Vec<PathBuf>, BlobStoreError> {
        // collect files recursively
        let mut paths = Vec::new();
        collect_entries(root, &mut paths)?;

        Ok(paths)
    }

    fn remove(&self, path: &Path) -> Result<(), BlobStoreError> {
        // remove existing file
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(BlobStoreError::from(error)),
        }
    }
}

/// Held WASI store lock file.
#[derive(Debug)]
struct WasiStoreLock {
    /// The lock file path.
    path: PathBuf,
}

impl WasiStoreLock {
    /// Acquire one store lock file.
    fn acquire(path: &Path) -> Result<Self, BlobStoreError> {
        // ensure lock directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // create lock file
        loop {
            match OpenOptions::new().write(true).create_new(true).open(path) {
                Ok(_file) => break,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    std::thread::sleep(STORE_LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(BlobStoreError::from(error)),
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for WasiStoreLock {
    fn drop(&mut self) {
        // release store lock
        cleanup_temp_path(&self.path);
    }
}

/// Collect blob files below one root.
fn collect_entries(root: &Path, paths: &mut Vec<PathBuf>) -> Result<(), BlobStoreError> {
    // read root directory
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(BlobStoreError::from(error)),
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

/// Create one temporary sibling file for atomic publishing.
fn create_temp_file(path: &Path) -> Result<(PathBuf, fs::File), BlobStoreError> {
    // extract file name
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
        // allocate unused sibling path
        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!("{file_name}.{counter}.tmp");
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

/// Remove one temporary or lock file.
fn cleanup_temp_path(path: &Path) {
    // remove file if present
    if let Err(error) = fs::remove_file(path)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::debug!("failed to clean blob file: {error}");
    }
}
