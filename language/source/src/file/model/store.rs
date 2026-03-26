use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;

use crate::{File, FileId, Uri};

/// Store of tracked files.
///
/// NOTE #Robustness: Updates across the lookup maps are not atomic yet.
#[derive(Debug)]
pub struct FileStore {
    /// The files by id.
    files_by_id: DashMap<FileId, Arc<File>>,
    /// The files by uri.
    files_by_uri: DashMap<Uri, FileId>,
    /// The files by path, only for files with valid paths.
    files_by_path: DashMap<PathBuf, FileId>,
}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStore {
    /// Create a new FileStore.
    pub fn new() -> Self {
        Self {
            files_by_id: DashMap::new(),
            files_by_uri: DashMap::new(),
            files_by_path: DashMap::new(),
        }
    }

    /// Insert a new file into the registry.
    ///
    /// # Panics
    /// Panics if a file with the same id already exists.
    pub fn insert(&self, file: File) {
        let id = file.id;
        assert!(
            !self.files_by_id.contains_key(&id),
            "file already exists for id: {id:?}"
        );

        self.files_by_uri.insert(file.uri.clone(), id);
        if let Some(path) = &file.path {
            self.files_by_path.insert(path.clone(), id);
        }
        self.files_by_id.insert(id, Arc::new(file));
    }

    /// Replace an existing file in the registry.
    ///
    /// The file id, uri, and path must match the existing file.
    /// This is used to replace a blank or unloaded file with its loaded content.
    ///
    /// # Panics
    /// Panics if no file with the given id exists.
    pub fn replace(&self, file: File) {
        let id = file.id;
        let existing = self
            .files_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("file does not exist for id: {id:?}"));
        let existing = existing.value();

        assert_eq!(
            existing.uri, file.uri,
            "file uri changed during replace for {id:?}"
        );
        assert_eq!(
            existing.path, file.path,
            "file path changed during replace for {id:?}"
        );

        // uri and path mappings stay the same, just update the file content
        self.files_by_id.insert(id, Arc::new(file));
    }

    /// Get a file by id.
    ///
    /// # Panics
    /// Panics if the file is not found.
    pub fn get(&self, id: FileId) -> Arc<File> {
        self.files_by_id
            .get(&id)
            .unwrap_or_else(|| panic!("file not found for id: {id:?}"))
            .clone()
    }

    /// Get a file by id, returning None if it is not found.
    pub fn get_maybe(&self, id: FileId) -> Option<Arc<File>> {
        self.files_by_id.get(&id).map(|entry| entry.clone())
    }

    /// Get a file id by its URI.
    pub fn get_id_by_uri(&self, uri: &Uri) -> Option<FileId> {
        self.files_by_uri.get(uri).map(|entry| *entry.value())
    }

    /// Get a file by uri.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<File>> {
        let id = self.get_id_by_uri(uri)?;
        Some(self.get(id))
    }

    /// Check if a file exists with the given URI.
    pub fn contains_uri(&self, uri: &Uri) -> bool {
        self.files_by_uri.contains_key(uri)
    }

    /// Get a file id by its path.
    pub fn get_id_by_path(&self, path: &Path) -> Option<FileId> {
        self.files_by_path.get(path).map(|entry| *entry.value())
    }

    /// Get a file by path.
    pub fn get_by_path(&self, path: &Path) -> Option<Arc<File>> {
        let id = self.get_id_by_path(path)?;
        Some(self.get(id))
    }

    /// Check if a file exists with the given path.
    pub fn contains_path(&self, path: &Path) -> bool {
        self.files_by_path.contains_key(path)
    }

    /// Get the number of files in the registry.
    pub fn len(&self) -> usize {
        self.files_by_id.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.files_by_id.is_empty()
    }
}
