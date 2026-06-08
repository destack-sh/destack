use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::repository::{Repository, RepositoryError, Revision};
use dashmap::DashMap;
use destack_source::{
    File, FileContent, FileContentEntry, FileContentId, FileId, FileMetadata, FileType, PathExt,
    StringId, Uri,
};
use im::OrdMap;
use rustc_hash::FxHashSet;

/// Normalize one logical file path.
pub(crate) fn normalize_logical_path(value: impl AsRef<str>) -> String {
    let value = value.as_ref();

    value.replace('\\', "/")
}

/// The file binding for one file in one revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct FileEntry {
    /// The logical path for this file in this revision.
    pub logical_path: StringId,
    /// The fixed content binding for this file.
    pub content_id: FileContentId,
}

impl FileEntry {
    /// Build one loaded file entry.
    pub(crate) fn loaded(logical_path: StringId, content_id: FileContentId) -> Self {
        Self {
            logical_path,
            content_id,
        }
    }
}

/// Shared immutable source content storage.
#[derive(Debug, Default)]
pub(crate) struct FileStore {
    /// Content payloads by exact content identity.
    content_by_id: DashMap<FileContentId, Arc<FileContentEntry>>,
}

impl FileStore {
    /// Create one empty content store.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Intern one content payload and return its exact identity.
    pub(crate) fn intern(&self, content: FileContent) -> FileContentId {
        let content_id = FileContentId::for_content(&content);

        self.content_by_id
            .entry(content_id)
            .or_insert_with(|| Arc::new(FileContentEntry::new(content)));

        content_id
    }

    /// Get one shared content payload.
    pub(crate) fn get(&self, content_id: FileContentId) -> Option<Arc<FileContentEntry>> {
        self.content_by_id
            .get(&content_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Retain only the reachable content ids.
    pub(crate) fn retain_reachable(&self, reachable: &HashSet<FileContentId>) {
        self.content_by_id
            .retain(|content_id, _| reachable.contains(content_id));
    }
}

impl Repository {
    /// Normalize one logical path for one workspace file path.
    pub fn logical_path(&self, path: &Path) -> String {
        // prefer the direct workspace relative path
        if let Ok(logical_path) = path.strip_prefix(&self.root) {
            return normalize_logical_path(logical_path.to_string_lossy());
        }

        // retry through canonical paths to collapse host path aliases
        if let Ok(canonical_root) = self.fs.canonicalize(&self.root)
            && let Ok(canonical_path) = self.fs.canonicalize(path)
            && let Ok(logical_path) = canonical_path.strip_prefix(&canonical_root)
        {
            return normalize_logical_path(logical_path.to_string_lossy());
        }

        normalize_logical_path(path.to_string_lossy())
    }

    /// Intern one normalized logical repository path.
    pub(crate) fn intern_logical_path(&self, logical_path: impl AsRef<str>) -> StringId {
        let logical_path = normalize_logical_path(logical_path);

        self.strings.intern(&logical_path)
    }

    /// Return one interned logical repository path as text.
    pub(crate) fn logical_path_text(&self, logical_path: StringId) -> &str {
        self.strings.get(logical_path)
    }

    /// Return the physical workspace path for one file entry.
    pub(crate) fn file_entry_path(&self, entry: &FileEntry) -> PathBuf {
        self.root.join(self.logical_path_text(entry.logical_path))
    }

    /// Build one file id for one workspace path.
    pub fn file_id(&self, path: &Path) -> FileId {
        // prefer immutable builtin files
        if let Some(file_id) = self.builtin.file_id_for_path(path) {
            return file_id;
        }

        // hash editable workspace paths
        let logical_path = self.logical_path(path);

        FileId::from_logical_str(&logical_path)
    }

    /// Return one source file for one revision and file id.
    pub fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(Arc::new(builtin.file())));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let Some(entry) = revision.file_entry(file_id) else {
            return Ok(None);
        };

        // assemble the physical source file
        let content = self.file_content_by_id(entry.content_id)?;
        let logical_path = self.logical_path_text(entry.logical_path);
        let path = self.file_entry_path(&entry);
        let uri = Uri::from_path(&path);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| logical_path.to_string());
        let file_type = FileType::from_path_or_unknown(&path);
        let file = File::from_content(file_id, name, uri, Some(path), file_type, content);

        Ok(Some(Arc::new(file)))
    }

    /// Return one workspace path metadata for one revision and workspace path.
    pub fn file_metadata(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileMetadata>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file_for_path(path) {
            return Ok(Some(builtin.metadata()));
        }

        // read editable file metadata
        let file_id = self.file_id(path);
        if let Some(file) = self.file(revision, file_id)? {
            return Ok(Some(FileMetadata::new(
                true,
                false,
                false,
                file.len as u64,
                None,
            )));
        }

        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();
        let normalized_path = path.normalize();
        let directory_paths = revision_cache
            .directory_paths
            .get_or_init(|| Arc::new(self.directory_paths_for_files(&revision_state.files)));

        // directory metadata
        if directory_paths.contains(&normalized_path) {
            return Ok(Some(FileMetadata::new(false, true, false, 0, None)));
        }

        Ok(None)
    }

    /// Return one file content identity from one revision.
    pub fn file_content_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<FileContentId>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(builtin.content_id()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;

        Ok(revision.file_content_id(file_id))
    }

    /// Return the logical path for one file in one revision.
    pub fn file_logical_path(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<String>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(builtin.uri.to_string()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let logical_path = revision
            .file_entry(file_id)
            .map(|entry| self.logical_path_text(entry.logical_path).to_string());

        Ok(logical_path)
    }

    /// Return editable file ids and interned logical paths for one revision.
    pub fn editable_file_logical_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<(FileId, StringId)>, RepositoryError> {
        let revision = self.revision(revision)?;
        let logical_paths = revision
            .files
            .iter()
            .map(|(file_id, entry)| (*file_id, entry.logical_path))
            .collect();

        Ok(logical_paths)
    }

    /// Return the file ids visible in one revision.
    pub fn file_ids(&self, revision: Revision) -> Result<Vec<FileId>, RepositoryError> {
        // start with editable revision files
        let revision = self.revision(revision)?;
        let mut file_ids = revision.files.keys().copied().collect::<Vec<_>>();

        // append immutable builtin files
        file_ids.extend(self.builtin.files().iter().map(|builtin| builtin.file_id()));
        file_ids.sort_unstable();
        file_ids.dedup();

        Ok(file_ids)
    }

    /// Build the workspace directory set for one file map.
    fn directory_paths_for_files(&self, files: &OrdMap<FileId, FileEntry>) -> FxHashSet<PathBuf> {
        let mut directories = FxHashSet::default();
        directories.insert(self.root.normalize());

        // physical workspace directories
        for entry in files.values() {
            let path = self.file_entry_path(entry);

            let mut current = path.parent().map(Path::to_path_buf);
            while let Some(directory) = current {
                let directory = directory.normalize();
                if !directory.starts_with(&self.root) {
                    break;
                }

                directories.insert(directory.clone());

                if directory == self.root {
                    break;
                }

                current = directory.parent().map(Path::to_path_buf);
            }
        }

        directories
    }
}
