use std::collections::HashMap;

use crate::{File, FileId, Uri};

/// Registry of files.
#[derive(Debug, Clone)]
pub struct FileRegistry {
    /// The files by id.
    files_by_id: HashMap<FileId, File>,
	/// The files by uri.
	files_by_uri: HashMap<Uri, FileId>,
    /// The next file id.
    next_file_id: u32 = 0,
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
            files_by_id: HashMap::new(),
            files_by_uri: HashMap::new(),
            next_file_id: 0,
        }
    }

    /// Get and increment the next file id.
    pub fn next_id(&mut self) -> FileId {
        let id = FileId::new(self.next_file_id);
        self.next_file_id += 1;
        id
    }

    /// Insert a file into the registry.
    pub fn insert(&mut self, file: File) {
        self.files_by_uri.insert(file.uri.clone(), file.id);
        self.files_by_id.insert(file.id, file);
    }

    /// Get a file by id.
    pub fn get(&self, id: FileId) -> Option<&File> {
        self.files_by_id.get(&id)
    }

    /// Get a file by uri.
    pub fn get_by_uri(&self, uri: Uri) -> Option<&File> {
        self.files_by_uri
            .get(&uri)
            .and_then(|id| self.files_by_id.get(id))
    }

    /// Iterate over the files in the registry.
    pub fn iter(&self) -> impl Iterator<Item = &File> {
        self.files_by_id.values()
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
