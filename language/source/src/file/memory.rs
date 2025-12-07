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
#[derive(Default, Clone)]
pub struct MemoryFileSystem {
    inner: Arc<RwLock<MemoryFileSystemState>>,
}

impl std::fmt::Debug for MemoryFileSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inenr = self.inner.read();
        let files_uris = inenr
            .files
            .keys()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>();
        let directories = inenr
            .directories
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>();
        f.debug_struct("MemoryFileSystem")
            .field("files", &files_uris)
            .field("directories", &directories)
            .finish()
    }
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
    /// Create a new (empty) MemoryFileSystem.
    pub fn new() -> Self {
        Self::default()
    }

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

    #[tracing::instrument(name = "fs.memory.exists", level = "trace", skip(self))]
    fn exists(&self, path: &Path) -> io::Result<bool> {
        tracing::trace!(?path, "fs.memory.exists");
        let inner = self.inner.read();
        Ok(inner.files.contains_key(path) || inner.directories.contains(path))
    }

    #[tracing::instrument(name = "fs.memory.metadata", level = "trace", skip(self))]
    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        tracing::trace!(?path, "fs.memory.metadata");
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

    #[tracing::instrument(name = "fs.memory.resolve_symlink", level = "trace", skip(self))]
    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        tracing::trace!(?path, "fs.memory.resolve_symlink");
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            path.display().to_string(),
        ))
    }

    #[tracing::instrument(name = "fs.memory.canonicalize", level = "trace", skip(self))]
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        tracing::trace!(?path, "fs.memory.canonicalize");
        let metadata = self.metadata(path)?;
        if metadata.is_directory || metadata.is_file {
            return Ok(path.normalize());
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            path.display().to_string(),
        ))
    }

    #[tracing::instrument(name = "fs.memory.read", level = "trace", skip(self))]
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        tracing::trace!(?path, "fs.memory.read");
        let inner = self.inner.read();
        inner
            .files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }

    #[tracing::instrument(name = "fs.memory.read_dir", level = "trace", skip(self))]
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        tracing::trace!(?path, "fs.memory.read_dir");
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

    #[tracing::instrument(name = "fs.memory.read_to_string", level = "trace", skip(self))]
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        tracing::trace!(?path, "fs.memory.read_to_string");
        let bytes = self.read(path)?;
        validate_utf8_string(bytes)
    }

    #[tracing::instrument(name = "fs.memory.symlink_metadata", level = "trace", skip(self))]
    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        tracing::trace!(?path, "fs.memory.symlink_metadata");
        self.metadata(path)
    }

    #[tracing::instrument(name = "fs.memory.write", level = "trace", skip(self, content))]
    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        tracing::trace!(?path, len = content.len(), "fs.memory.write");
        self.add_file(path, content)
    }

    #[tracing::instrument(name = "fs.memory.create_dir", level = "trace", skip(self))]
    fn create_dir(&self, path: &Path) -> io::Result<()> {
        tracing::trace!(?path, "fs.memory.create_dir");
        let mut inner = self.inner.write();

        // check parent exists (root "/" is always valid as a parent)
        if let Some(parent) = path.parent() {
            let parent_is_root = parent == Path::new("/") || parent.as_os_str().is_empty();
            if !parent_is_root && !inner.directories.contains(parent) {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("parent directory does not exist: {}", parent.display()),
                ));
            }
        }

        // check not already exists
        if inner.directories.contains(path) || inner.files.contains_key(path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                path.display().to_string(),
            ));
        }

        inner.directories.insert(path.to_path_buf());
        Ok(())
    }

    #[tracing::instrument(name = "fs.memory.create_dir_all", level = "trace", skip(self))]
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        tracing::trace!(?path, "fs.memory.create_dir_all");
        let mut inner = self.inner.write();

        // insert all ancestors
        for ancestor in path.ancestors() {
            if ancestor.as_os_str().is_empty() {
                continue;
            }
            inner.directories.insert(ancestor.to_path_buf());
        }

        // insert root if absolute
        if path.is_absolute() {
            inner.directories.insert(PathBuf::from("/"));
        }

        Ok(())
    }

    #[tracing::instrument(name = "fs.memory.remove_file", level = "trace", skip(self))]
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        tracing::trace!(?path, "fs.memory.remove_file");
        let mut inner = self.inner.write();
        if inner.files.remove(path).is_some() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ))
        }
    }

    #[tracing::instrument(name = "fs.memory.remove_dir", level = "trace", skip(self))]
    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        tracing::trace!(?path, "fs.memory.remove_dir");
        let mut inner = self.inner.write();

        // check directory exists
        if !inner.directories.contains(path) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                path.display().to_string(),
            ));
        }

        // check directory is empty (no files or subdirectories)
        let has_files = inner.files.keys().any(|p| p.parent() == Some(path));
        let has_subdirs = inner
            .directories
            .iter()
            .any(|p| p != path && p.parent() == Some(path));

        if has_files || has_subdirs {
            return Err(io::Error::new(
                io::ErrorKind::DirectoryNotEmpty,
                path.display().to_string(),
            ));
        }

        inner.directories.remove(path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read_file() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/test/file.txt");

        fs.write(path, b"hello world").unwrap();

        assert!(fs.exists(path).unwrap());
        assert_eq!(fs.read(path).unwrap(), b"hello world");
        assert_eq!(fs.read_to_string(path).unwrap(), "hello world");
    }

    #[test]
    fn test_write_string() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/test/file.txt");

        fs.write_string(path, "hello world").unwrap();

        assert_eq!(fs.read_to_string(path).unwrap(), "hello world");
    }

    #[test]
    fn test_write_overwrites_existing() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/test/file.txt");

        fs.write(path, b"first").unwrap();
        fs.write(path, b"second").unwrap();

        assert_eq!(fs.read(path).unwrap(), b"second");
    }

    #[test]
    fn test_write_creates_parent_directories() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/a/b/c/file.txt");

        fs.write(path, b"content").unwrap();

        assert!(fs.exists(Path::new("/a")).unwrap());
        assert!(fs.exists(Path::new("/a/b")).unwrap());
        assert!(fs.exists(Path::new("/a/b/c")).unwrap());
        assert!(fs.metadata(Path::new("/a/b/c")).unwrap().is_directory);
    }

    #[test]
    fn test_create_dir() {
        let fs = MemoryFileSystem::new();

        // create root first
        fs.create_dir(Path::new("/root")).unwrap();
        assert!(fs.exists(Path::new("/root")).unwrap());
        assert!(fs.metadata(Path::new("/root")).unwrap().is_directory);

        // create nested (parent must exist)
        fs.create_dir(Path::new("/root/child")).unwrap();
        assert!(fs.exists(Path::new("/root/child")).unwrap());
    }

    #[test]
    fn test_create_dir_fails_without_parent() {
        let fs = MemoryFileSystem::new();

        let result = fs.create_dir(Path::new("/a/b/c"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_create_dir_fails_if_exists() {
        let fs = MemoryFileSystem::new();

        fs.create_dir(Path::new("/test")).unwrap();
        let result = fs.create_dir(Path::new("/test"));

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn test_create_dir_all() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/a/b/c/d");

        fs.create_dir_all(path).unwrap();

        assert!(fs.exists(Path::new("/a")).unwrap());
        assert!(fs.exists(Path::new("/a/b")).unwrap());
        assert!(fs.exists(Path::new("/a/b/c")).unwrap());
        assert!(fs.exists(path).unwrap());
    }

    #[test]
    fn test_create_dir_all_idempotent() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/a/b/c");

        fs.create_dir_all(path).unwrap();
        fs.create_dir_all(path).unwrap(); // should not fail

        assert!(fs.exists(path).unwrap());
    }

    #[test]
    fn test_remove_file() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/test/file.txt");

        fs.write(path, b"content").unwrap();
        assert!(fs.exists(path).unwrap());

        fs.remove_file(path).unwrap();
        assert!(!fs.exists(path).unwrap());

        // parent directory should still exist
        assert!(fs.exists(Path::new("/test")).unwrap());
    }

    #[test]
    fn test_remove_file_not_found() {
        let fs = MemoryFileSystem::new();

        let result = fs.remove_file(Path::new("/nonexistent"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_remove_dir() {
        let fs = MemoryFileSystem::new();

        fs.create_dir_all(Path::new("/a/b")).unwrap();
        fs.remove_dir(Path::new("/a/b")).unwrap();

        assert!(!fs.exists(Path::new("/a/b")).unwrap());
        assert!(fs.exists(Path::new("/a")).unwrap());
    }

    #[test]
    fn test_remove_dir_not_empty() {
        let fs = MemoryFileSystem::new();

        fs.write(Path::new("/test/file.txt"), b"content").unwrap();

        let result = fs.remove_dir(Path::new("/test"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::DirectoryNotEmpty);
    }

    #[test]
    fn test_remove_dir_with_subdirectory() {
        let fs = MemoryFileSystem::new();

        fs.create_dir_all(Path::new("/a/b")).unwrap();

        let result = fs.remove_dir(Path::new("/a"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::DirectoryNotEmpty);
    }

    #[test]
    fn test_remove_dir_not_found() {
        let fs = MemoryFileSystem::new();

        let result = fs.remove_dir(Path::new("/nonexistent"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_roundtrip_binary_content() {
        let fs = MemoryFileSystem::new();
        let path = Path::new("/binary.bin");
        let content: Vec<u8> = (0..=255).collect();

        fs.write(path, &content).unwrap();

        assert_eq!(fs.read(path).unwrap(), content);
    }
}
