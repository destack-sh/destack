use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::{File, FileId, Uri};

/// Registry of files. THREAD-SAFE.
/// NOTE #Robustness: use single/exctra lock around both id<->file and uri<->id mappings?
#[derive(Debug)]
pub struct FileRegistry {
    /// The files by id.
    files_by_id: DashMap<FileId, Arc<File>>,
    /// The files by uri.
    files_by_uri: DashMap<Uri, FileId>,
    /// The next file id.
    next_file_id: AtomicU32,
}

impl Default for FileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FileRegistry {
    /// Create a new FileRegistry.
    pub fn new() -> Self {
        Self {
            files_by_id: DashMap::new(),
            files_by_uri: DashMap::new(),
            next_file_id: AtomicU32::new(0),
        }
    }

    /// Get and increment the next file id.
    pub fn next_id(&self) -> FileId {
        let next_file_id = self.next_file_id.fetch_add(1, Ordering::Relaxed);
        FileId::new(next_file_id)
    }

    /// Insert a file into the registry.
    pub fn insert(&self, file: File) {
        self.files_by_uri.insert(file.uri.clone(), file.id);
        self.files_by_id.insert(file.id, Arc::new(file));
    }

    /// Get a file by id.
    pub fn get(&self, id: FileId) -> Option<Arc<File>> {
        self.files_by_id.get(&id).map(|file| file.clone())
    }

    /// Get a file by uri.
    pub fn get_by_uri(&self, uri: &Uri) -> Option<Arc<File>> {
        self.files_by_uri
            .get(uri)
            .and_then(|id| self.files_by_id.get(&id).map(|file| file.clone()))
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
