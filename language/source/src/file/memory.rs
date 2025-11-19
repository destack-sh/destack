use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::validate_utf8_string;

use super::FileSystem;
use super::metadata::FileMetadata;
use super::path::PathExt;

/// Memory file system implementation. THREAD-SAFE.
#[derive(Debug, Default, Clone)]
pub struct MemoryFileSystem {
    inner: Arc<RwLock<MemoryFileSystemState>>,
}

/// State of the MemoryFileSystem.
#[derive(Debug, Default, Clone)]
struct MemoryFileSystemState {
    /// Files by path.
    files: HashMap<PathBuf, Vec<u8>>,
    /// Directories by path.
    directories: HashSet<PathBuf>,
}

impl MemoryFileSystem {
    /// Creates a new MemoryFileSystem from the given files.
    /// Any intermediate directories are created automatically.
    pub fn from_files(entries: &[(&str, &str)]) -> Self {
        let fs = Self::default();
        for (path, content) in entries {
            fs.add_file(Path::new(path), content.as_bytes())
                .expect("failed to add file");
        }
        fs
    }

    /// Adds a file to the MemoryFileSystem.
    pub fn add_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> io::Result<()> {
        let path = path.as_ref();
        let mut inner = self.inner.write().expect("lock poisoned");
        Self::ensure_directory_entries(&mut inner.directories, path);
        inner.files.insert(path.to_path_buf(), content.to_vec());
        Ok(())
    }

    /// Ensures that all directories leading up to the given path are created.
    fn ensure_directory_entries(directories: &mut HashSet<PathBuf>, path: &Path) {
        for ancestor in path.ancestors().skip(1) {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            directories.insert(ancestor.to_path_buf());
        }
        if path.is_absolute() {
            directories.insert(PathBuf::from("/"));
        }
    }
}

impl FileSystem for MemoryFileSystem {
    fn new() -> Self {
        Self::default()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let inner = self.inner.read().expect("lock poisoned");
        inner
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        validate_utf8_string(bytes)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let inner = self.inner.read().expect("lock poisoned");
        if inner.directories.contains(path) {
            Ok(FileMetadata::new(false, true, false))
        } else if inner.files.contains_key(path) {
            Ok(FileMetadata::new(true, false, false))
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ))
        }
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata(path)
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            path.display().to_string(),
        ))
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let metadata = self.metadata(path)?;
        if metadata.is_directory || metadata.is_file {
            return Ok(path.normalize());
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            path.display().to_string(),
        ))
    }
}
