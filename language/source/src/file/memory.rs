use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;

use crate::validate_utf8_string;

use super::FileSystem;
use super::path::PathExt;
use super::system::FileMetadata;

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
    /// Create a new MemoryFileSystem from the given files.
    /// Any intermediate directories are created automatically.
    pub fn from_files(entries: &[(&str, &str)]) -> Self {
        let fs = Self::default();
        for (path, content) in entries {
            fs.add_file(Path::new(path), content.as_bytes())
                .expect("failed to add file");
        }
        fs
    }

    /// Add a file to the MemoryFileSystem (including all parent directories).
    pub fn add_file<P: AsRef<Path>>(&self, path: P, content: &[u8]) -> io::Result<()> {
        let path = path.as_ref();
        self.ensure_directories(path);
        let mut inner = self.inner.write();
        inner.files.insert(path.to_path_buf(), content.to_vec());
        Ok(())
    }

    /// Ensure that all directories leading up to the given path are created.
    fn ensure_directories(&self, path: &Path) {
        let mut inner = self.inner.write();
        for ancestor in path.ancestors().skip(1) {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            inner.directories.insert(ancestor.to_path_buf());
        }
        if path.is_absolute() {
            inner.directories.insert(PathBuf::from("/"));
        }
    }
}

impl FileSystem for MemoryFileSystem {
    fn new() -> Self {
        Self::default()
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        let inner = self.inner.read();
        Ok(inner.files.contains_key(path) || inner.directories.contains(path))
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let inner = self.inner.read();
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

    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
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

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let inner = self.inner.read();
        inner
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let inner = self.inner.read();
        if !inner.directories.contains(path) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ));
        }
        let mut entries = Vec::new();
        for file_path in inner.files.keys() {
            if file_path.parent() == Some(path) {
                entries.push(file_path.clone());
            }
        }
        for dir_path in &inner.directories {
            if dir_path.parent() == Some(path) {
                entries.push(dir_path.clone());
            }
        }
        entries.sort();
        Ok(entries)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let bytes = self.read(path)?;
        validate_utf8_string(bytes)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        self.metadata(path)
    }
}
