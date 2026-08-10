use std::path::Path;
use std::sync::Arc;

use destack_repository::{Repository, Revision};
use destack_source::{File, FileId, Uri};

use crate::Error;
use crate::workspace::Workspace;

/// One currently open file tracked by the workspace.
#[derive(Debug, Clone)]
pub(crate) struct OpenFile {
    /// Client-facing uri for this file.
    pub uri: Uri,
    /// Client-provided file version.
    pub version: i32,
    /// Current repository file.
    pub file: Arc<File>,
}

impl OpenFile {
    /// Return the client version when one revision contains this open Blob.
    pub(crate) fn version_at(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<i32>, Error> {
        let Some(blob) = repository.file_blob(revision, file_id)? else {
            return Ok(None);
        };

        Ok((blob == self.file.blob).then_some(self.version))
    }
}

impl Workspace {
    /// Return the open file for one path.
    pub(crate) fn find_open_file(&self, path: &Path) -> Option<OpenFile> {
        // open file paths are stored by normalized path
        let path = self.normalized_path(path);

        self.open_file_by_path
            .get(path.as_path())
            .map(|file| file.value().clone())
    }

    /// Insert or replace one open file.
    pub(crate) fn upsert_open_file(&self, path: &Path, file: OpenFile) {
        let path = self.normalized_path(path);

        // mirror open content into the shared filesystem overlay
        if let Some(overlay_file_system) = self.overlay_file_system.as_ref() {
            if file.file.ty.is_binary() {
                overlay_file_system.set_overlay_bytes(path.as_path(), file.file.bytes().to_vec());
            } else {
                overlay_file_system.set_overlay(path.as_path(), file.file.text().to_string());
            }
        }

        // store protocol state after overlay update succeeds
        self.open_file_by_path.insert(path, file);
    }

    /// Remove one open file.
    pub(crate) fn remove_open_file(&self, path: &Path) -> Option<OpenFile> {
        // remove the overlay before dropping the open file
        let path = self.normalized_path(path);

        if let Some(overlay_file_system) = self.overlay_file_system.as_ref() {
            overlay_file_system.remove_overlay(path.as_path());
        }

        self.open_file_by_path
            .remove(path.as_path())
            .map(|(_, file)| file)
    }

    /// Return true when a path is open.
    pub fn has_open_file(&self, path: &Path) -> bool {
        // compare normalized paths with the open file map
        let path = self.normalized_path(path);

        self.open_file_by_path.contains_key(path.as_path())
    }

    /// Return every open file.
    pub(crate) fn open_files(&self) -> Vec<(std::path::PathBuf, OpenFile)> {
        self.open_file_by_path
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Remove every open file.
    pub(crate) fn remove_open_files(&self) {
        let files = self.open_files();

        for (path, _) in files {
            self.remove_open_file(path.as_path());
        }
    }
}
