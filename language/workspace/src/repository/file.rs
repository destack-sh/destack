use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_source::{
    File, FileContent, FileContentEntry, FileContentId, FileId, FileMetadata, FileType, PathExt,
    Uri,
};
use im::OrdMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use crate::repository::{Repository, RepositoryError, Revision};

const WORKSPACE_FILE_SOURCE_TAG: u8 = 0;
const BUILTIN_FILE_SOURCE_TAG: u8 = 1;

/// The source for one file in one revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum FileSource {
    /// One workspace file keyed by logical path.
    Workspace { logical_path: String },
    /// One builtin file keyed by builtin module path.
    Builtin { logical_path: String },
}

impl FileSource {
    /// Build one workspace file source.
    pub(crate) fn workspace(logical_path: impl Into<String>) -> Self {
        Self::Workspace {
            logical_path: logical_path.into(),
        }
    }

    /// Build one builtin file source.
    pub(crate) fn builtin(logical_path: impl Into<String>) -> Self {
        Self::Builtin {
            logical_path: logical_path.into(),
        }
    }
}

/// Normalize one logical file path string.
pub(crate) fn normalize_logical_path_str(value: &str) -> String {
    value.replace('\\', "/")
}

/// Normalize one logical file path.
pub(crate) fn normalize_logical_path(path: &Path) -> String {
    normalize_logical_path_str(&path.to_string_lossy())
}

/// The file binding for one file in one revision.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct FileEntry {
    /// The source for this file in this revision.
    pub source: FileSource,
    /// The fixed content binding for this file.
    pub content_id: FileContentId,
}

impl FileEntry {
    /// Build one loaded file entry.
    pub(crate) fn loaded(source: FileSource, content_id: FileContentId) -> Self {
        Self { source, content_id }
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
    /// Build one structured file id payload for one file source.
    fn file_id_bytes_for_source(source: &FileSource) -> Vec<u8> {
        let mut bytes = Vec::new();

        // source namespace
        match source {
            FileSource::Workspace { logical_path } => {
                bytes.push(WORKSPACE_FILE_SOURCE_TAG);
                Self::push_normalized_logical_path(&mut bytes, logical_path);
            }
            FileSource::Builtin { logical_path } => {
                bytes.push(BUILTIN_FILE_SOURCE_TAG);
                Self::push_normalized_logical_path(&mut bytes, logical_path);
            }
        }

        bytes
    }

    /// Push one normalized logical path into one file id payload.
    fn push_normalized_logical_path(bytes: &mut Vec<u8>, logical_path: &str) {
        let logical_path = normalize_logical_path_str(logical_path);
        bytes.extend_from_slice(logical_path.as_bytes());
    }

    /// Normalize one logical path for one workspace file path.
    pub fn normalize_workspace_path(&self, path: &Path) -> String {
        // prefer the direct workspace relative path
        if let Ok(logical_path) = path.strip_prefix(&self.root) {
            return normalize_logical_path(logical_path);
        }

        // retry through canonical paths to collapse host path aliases
        if let Ok(canonical_root) = self.fs.canonicalize(&self.root)
            && let Ok(canonical_path) = self.fs.canonicalize(path)
            && let Ok(logical_path) = canonical_path.strip_prefix(&canonical_root)
        {
            return normalize_logical_path(logical_path);
        }

        normalize_logical_path(path)
    }

    /// Build one file id for one explicit file source.
    pub(crate) fn file_id_for_source(&self, source: &FileSource) -> FileId {
        let bytes = Self::file_id_bytes_for_source(source);
        FileId::from_source_bytes(&bytes)
    }

    /// Build one file id for one workspace file path.
    pub fn file_id_for_workspace_path(&self, path: &Path) -> FileId {
        let logical_path = self.normalize_workspace_path(path);
        let source = FileSource::workspace(logical_path);

        self.file_id_for_source(&source)
    }

    /// Return one source file for one revision and file id.
    pub fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        let revision = self.revision(revision)?;
        let Some(entry) = revision.file_entry(file_id) else {
            return Ok(self.builtin_file(file_id));
        };
        let content = self.file_content_by_id(entry.content_id)?;
        let file = self.build_file_at_revision(file_id, &entry.source, content);

        Ok(Some(Arc::new(file)))
    }

    /// Return one workspace file for one revision and workspace path.
    pub fn file_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);
        self.file(revision, file_id)
    }

    /// Return one workspace path metadata for one revision and workspace path.
    pub fn metadata(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileMetadata>, RepositoryError> {
        // file metadata
        if let Some(file) = self.file_for_path(revision, path)? {
            return Ok(Some(FileMetadata::new(
                true,
                false,
                false,
                file.len as u64,
                None,
            )));
        }

        let revision_state = self.revision(revision)?;
        let revision_cache = self.revision_cache(revision);
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
        let revision = self.revision(revision)?;
        Ok(revision
            .file_content_id(file_id)
            .or_else(|| self.builtin_file_content_id(file_id)))
    }

    /// Build the workspace directory set for one file map.
    fn directory_paths_for_files(&self, files: &OrdMap<FileId, FileEntry>) -> FxHashSet<PathBuf> {
        let mut directories = FxHashSet::default();
        directories.insert(self.root.normalize());

        // physical workspace directories
        for entry in files.values() {
            let Some(path) = self.path_for_source(&entry.source) else {
                continue;
            };

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

    /// Build one file view from one revision file binding.
    fn build_file_at_revision(
        &self,
        file_id: FileId,
        source: &FileSource,
        content: Arc<FileContentEntry>,
    ) -> File {
        let path = self.path_for_source(source);
        let uri = self.uri_for_source(source);
        let name = self.name_for_source(source, path.as_deref());
        let file_type = self.file_type_for_source(source, path.as_deref());

        File::from_content(file_id, name, uri, path, file_type, content)
    }

    /// Materialize one physical path for one revision source when possible.
    pub(crate) fn path_for_source(&self, source: &FileSource) -> Option<PathBuf> {
        match source {
            FileSource::Workspace { logical_path } => Some(self.root.join(logical_path)),
            FileSource::Builtin { .. } => None,
        }
    }

    /// Return the uri for one file source.
    fn uri_for_source(&self, source: &FileSource) -> Uri {
        match source {
            FileSource::Workspace { logical_path } => Uri::from_path(&self.root.join(logical_path)),
            FileSource::Builtin { logical_path } => {
                Uri::from_string(format!("builtin://{logical_path}"))
            }
        }
    }

    /// Return the display name for one file source.
    fn name_for_source(&self, source: &FileSource, path: Option<&Path>) -> String {
        match source {
            FileSource::Workspace { logical_path } | FileSource::Builtin { logical_path } => path
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| logical_path.clone()),
        }
    }

    /// Return the file type for one file source.
    fn file_type_for_source(&self, source: &FileSource, path: Option<&Path>) -> FileType {
        match source {
            FileSource::Workspace { logical_path } | FileSource::Builtin { logical_path } => path
                .map(FileType::from_path_or_unknown)
                .unwrap_or_else(|| FileType::from_path_or_unknown(Path::new(logical_path))),
        }
    }
}
