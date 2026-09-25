use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;

use super::{FileMetadata, FileSystem};

/// Filesystem that overlays in-memory content on top of another filesystem.
///
/// When a path has overlay content set, all reads return that content.
///
/// Writes publish to the underlying filesystem and then clear the overlay.
#[derive(Debug)]
pub struct OverlayFileSystem {
    /// The underlying filesystem to delegate to.
    inner: Arc<dyn FileSystem>,
    /// Overlay content keyed by canonical path.
    overlays: DashMap<PathBuf, OverlayEntry>,
}

/// An entry in the overlay map.
#[derive(Debug, Clone)]
struct OverlayEntry {
    /// The content as bytes.
    content: Vec<u8>,
}

impl OverlayFileSystem {
    /// Create a new overlay filesystem wrapping the given inner filesystem.
    pub fn with_inner(inner: Arc<dyn FileSystem>) -> Self {
        Self {
            inner,
            overlays: DashMap::new(),
        }
    }

    /// Set overlay content for a path.
    ///
    /// Subsequent reads will return this content instead of the underlying file.
    pub fn set_overlay(&self, path: &Path, content: String) {
        let canonical = self.normalize_path(path);
        self.overlays.insert(
            canonical,
            OverlayEntry {
                content: content.into_bytes(),
            },
        );
    }

    /// Set overlay content as bytes for a path.
    pub fn set_overlay_bytes(&self, path: &Path, content: Vec<u8>) {
        let canonical = self.normalize_path(path);
        self.overlays.insert(canonical, OverlayEntry { content });
    }

    /// Remove overlay content for a path.
    ///
    /// Subsequent reads will delegate to the underlying filesystem.
    pub fn remove_overlay(&self, path: &Path) {
        let canonical = self.normalize_path(path);
        self.overlays.remove(&canonical);
    }

    /// Check if a path has overlay content.
    pub fn has_overlay(&self, path: &Path) -> bool {
        let canonical = self.normalize_path(path);
        self.overlays.contains_key(&canonical)
    }

    /// Clear all overlays.
    pub fn clear_overlays(&self) {
        self.overlays.clear();
    }

    /// Get the number of active overlays.
    pub fn overlay_count(&self) -> usize {
        self.overlays.len()
    }

    /// Normalize a path for use as overlay key.
    fn normalize_path(&self, path: &Path) -> PathBuf {
        // try to canonicalize via inner fs, fall back to the path as-is
        self.inner
            .canonicalize(path)
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

impl FileSystem for OverlayFileSystem {
    fn new() -> Self
    where
        Self: Sized,
    {
        // default to wrapping physical filesystem
        Self::with_inner(Arc::new(super::PhysicalFileSystem::new()))
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        let canonical = self.normalize_path(path);
        if self.overlays.contains_key(&canonical) {
            return Ok(true);
        }
        self.inner.exists(path)
    }

    fn metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let canonical = self.normalize_path(path);
        if let Some(entry) = self.overlays.get(&canonical) {
            // overlay files are always regular files
            return Ok(FileMetadata::new(
                true,
                false,
                false,
                entry.content.len() as u64,
                None,
            ));
        }
        self.inner.metadata(path)
    }

    fn resolve_symlink(&self, path: &Path) -> io::Result<PathBuf> {
        // overlays don't support symlinks, delegate to inner
        self.inner.resolve_symlink(path)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        self.inner.canonicalize(path)
    }

    fn open(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        let canonical = self.normalize_path(path);
        if let Some(entry) = self.overlays.get(&canonical) {
            let bytes = entry.content.clone();

            return Ok(Box::new(io::Cursor::new(bytes)));
        }

        self.inner.open(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let canonical = self.normalize_path(path);
        if let Some(entry) = self.overlays.get(&canonical) {
            return Ok(entry.content.clone());
        }
        self.inner.read(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        // gather and de duplicate entries from inner and overlays
        let normalized_directory = self.normalize_path(path);
        let mut entries = Vec::new();
        let mut normalized_entries = HashSet::<PathBuf>::new();

        // read inner directory entries first
        let inner_result = self.inner.read_dir(path);
        if let Ok(inner_entries) = &inner_result {
            for entry in inner_entries {
                let normalized_entry = self.normalize_path(entry);
                if normalized_entries.insert(normalized_entry) {
                    entries.push(entry.clone());
                }
            }
        }

        // then merge overlays that are direct children of this directory
        let mut overlay_entries = Vec::new();
        for entry in self.overlays.iter() {
            let overlay_path = entry.key();
            let Some(parent) = overlay_path.parent() else {
                continue;
            };

            let normalized_parent = self.normalize_path(parent);
            if normalized_parent == normalized_directory {
                overlay_entries.push(overlay_path.clone());
            }
        }
        overlay_entries.sort();

        // merge overlay entries
        for overlay_path in overlay_entries {
            let normalized_entry = self.normalize_path(&overlay_path);
            if normalized_entries.insert(normalized_entry) {
                entries.push(overlay_path);
            }
        }

        // if the inner filesystem errored, only allow NotFound when overlays provide entries
        if let Err(err) = inner_result
            && (err.kind() != io::ErrorKind::NotFound || entries.is_empty())
        {
            return Err(err);
        }

        Ok(entries)
    }

    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        let canonical = self.normalize_path(path);
        if let Some(entry) = self.overlays.get(&canonical) {
            return String::from_utf8(entry.content.clone())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
        }
        self.inner.read_to_string(path)
    }

    fn symlink_metadata(&self, path: &Path) -> io::Result<FileMetadata> {
        let canonical = self.normalize_path(path);
        if let Some(entry) = self.overlays.get(&canonical) {
            // overlay files are always regular files, not symlinks
            return Ok(FileMetadata::new(
                true,
                false,
                false,
                entry.content.len() as u64,
                None,
            ));
        }
        self.inner.symlink_metadata(path)
    }

    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        // retain the overlay unless its underlying write succeeds
        self.inner.write(path, content)?;
        let canonical = self.normalize_path(path);
        self.overlays.remove(&canonical);

        Ok(())
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        // removing also clears overlay
        let canonical = self.normalize_path(path);
        self.overlays.remove(&canonical);
        self.inner.remove_file(path)
    }

    fn remove_dir(&self, path: &Path) -> io::Result<()> {
        self.inner.remove_dir(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::MemoryFileSystem;

    fn test_fs() -> OverlayFileSystem {
        let inner = Arc::new(MemoryFileSystem::new());
        inner
            .add_file(Path::new("/test.tspp"), b"disk content")
            .unwrap();
        OverlayFileSystem::with_inner(inner)
    }

    #[test]
    fn test_read_without_overlay_delegates_to_inner() {
        // reads delegate to inner when no overlay exists
        let fs = test_fs();
        let content = fs.read_to_string(Path::new("/test.tspp")).unwrap();
        assert_eq!(content, "disk content");
    }

    #[test]
    fn test_read_with_overlay_returns_overlay_content() {
        // reads return overlay content
        let fs = test_fs();
        fs.set_overlay(Path::new("/test.tspp"), "overlay content".to_string());
        let content = fs.read_to_string(Path::new("/test.tspp")).unwrap();
        assert_eq!(content, "overlay content");
    }

    #[test]
    fn test_remove_overlay_restores_inner_content() {
        // removing an overlay restores inner content
        let fs = test_fs();
        fs.set_overlay(Path::new("/test.tspp"), "overlay content".to_string());
        fs.remove_overlay(Path::new("/test.tspp"));
        let content = fs.read_to_string(Path::new("/test.tspp")).unwrap();
        assert_eq!(content, "disk content");
    }

    #[test]
    fn test_overlay_for_nonexistent_file() {
        // overlays can provide content for a path not present in inner
        let fs = test_fs();
        assert!(!fs.exists(Path::new("/new.tspp")).unwrap());
        fs.set_overlay(Path::new("/new.tspp"), "new content".to_string());
        assert!(fs.exists(Path::new("/new.tspp")).unwrap());
        let content = fs.read_to_string(Path::new("/new.tspp")).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn test_write_clears_overlay() {
        // writes go to inner and clear any overlay for that path
        let fs = test_fs();
        fs.set_overlay(Path::new("/test.tspp"), "overlay content".to_string());
        fs.write(Path::new("/test.tspp"), b"written content")
            .unwrap();
        assert!(!fs.has_overlay(Path::new("/test.tspp")));
        let content = fs.read_to_string(Path::new("/test.tspp")).unwrap();
        assert_eq!(content, "written content");
    }

    #[test]
    fn test_metadata_reflects_overlay() {
        // overlay paths report regular file metadata
        let fs = test_fs();
        fs.set_overlay(Path::new("/test.tspp"), "short".to_string());
        let meta = fs.metadata(Path::new("/test.tspp")).unwrap();
        assert!(meta.is_file);
        assert!(!meta.is_directory);
    }

    #[test]
    fn test_read_dir_merges_overlay_entries_with_inner_results() {
        // read_dir returns a union of inner and overlay direct children
        let inner = Arc::new(MemoryFileSystem::new());
        inner
            .add_file(Path::new("/dir/disk.tspp"), b"disk")
            .unwrap();
        inner
            .add_file(Path::new("/dir/inner_only.tspp"), b"disk")
            .unwrap();
        let fs = OverlayFileSystem::with_inner(inner);

        fs.set_overlay(Path::new("/dir/overlay.tspp"), "overlay".to_string());
        fs.set_overlay(Path::new("/dir/disk.tspp"), "overlay".to_string());

        let mut entries = fs.read_dir(Path::new("/dir")).unwrap();
        entries.sort();

        assert_eq!(
            entries,
            vec![
                PathBuf::from("/dir/disk.tspp"),
                PathBuf::from("/dir/inner_only.tspp"),
                PathBuf::from("/dir/overlay.tspp"),
            ]
        );
    }

    #[test]
    fn test_read_dir_returns_overlay_entries_when_inner_dir_is_missing() {
        // read_dir succeeds if inner returns NotFound but overlays exist in that directory
        let inner = Arc::new(MemoryFileSystem::new());
        let fs = OverlayFileSystem::with_inner(inner);

        fs.set_overlay(Path::new("/missing/overlay.tspp"), "overlay".to_string());

        let mut entries = fs.read_dir(Path::new("/missing")).unwrap();
        entries.sort();
        assert_eq!(entries, vec![PathBuf::from("/missing/overlay.tspp")]);
    }
}
