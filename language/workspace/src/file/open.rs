use std::path::{Path, PathBuf};

use destack_repository::{Repository, Revision};
use destack_source::{FileContentId, FileId, Uri};

use crate::FileChange;
use crate::diagnostic::Error;
use crate::workspace::Workspace;

/// One currently open file tracked by the workspace.
#[derive(Debug, Clone)]
pub(crate) struct OpenFile {
    /// Client-facing uri for this file.
    pub uri: Uri,
    /// Client-provided file version.
    pub version: i32,
    /// Repository content id corresponding to the open content.
    pub content_id: FileContentId,
    /// Current open content.
    pub content: FileChange,
}

impl Workspace {
    /// Return the open file for one path.
    pub(crate) fn open_state(&self, path: &Path) -> Option<OpenFile> {
        // open file paths are stored by normalized path
        let path = Self::normalized_path(path);

        self.open_file_by_path
            .get(path.as_path())
            .map(|file| file.value().clone())
    }

    /// Return the client version for one open file path.
    pub(crate) fn open_file_version(&self, path: &Path) -> Option<i32> {
        self.open_state(path).map(|file| file.version)
    }

    /// Set one open file.
    pub(crate) fn set_open_state(
        &self,
        path: &Path,
        uri: Uri,
        version: i32,
        content_id: FileContentId,
        content: FileChange,
    ) {
        // mirror open text into the shared filesystem overlay
        let path = Self::normalized_path(path);
        let file = OpenFile {
            uri,
            version,
            content_id,
            content: content.clone(),
        };

        // update the text overlay only for text content
        if let Some(overlay_file_system) = self.overlay_file_system.as_ref() {
            match content {
                FileChange::Text { content } => {
                    overlay_file_system.set_overlay(path.as_path(), content);
                }
                FileChange::Bytes { .. } | FileChange::Removed => {
                    overlay_file_system.remove_overlay(path.as_path());
                }
            }
        }

        // store protocol state after overlay update succeeds
        self.open_file_by_path.insert(path, file);
    }

    /// Remove one open file.
    pub(crate) fn remove_open_state(&self, path: &Path) -> Option<OpenFile> {
        // remove overlay state before dropping open file metadata
        let path = Self::normalized_path(path);

        if let Some(overlay_file_system) = self.overlay_file_system.as_ref() {
            overlay_file_system.remove_overlay(path.as_path());
        }

        self.open_file_by_path
            .remove(path.as_path())
            .map(|(_, file)| file)
    }

    /// Return the current text for one open file.
    pub(crate) fn open_file_text(&self, path: &Path) -> Option<String> {
        let file = self.open_state(path)?;
        let FileChange::Text { content } = file.content else {
            return None;
        };

        Some(content)
    }

    /// Return true when a path is open.
    pub fn has_open_file(&self, path: &Path) -> bool {
        // compare normalized paths with the open file map
        let path = Self::normalized_path(path);

        self.open_file_by_path.contains_key(path.as_path())
    }

    /// Return open files contained by one root.
    pub(crate) fn open_files_under(&self, root: &Path) -> Vec<(PathBuf, OpenFile)> {
        self.open_file_by_path
            .iter()
            .filter(|entry| entry.key().starts_with(root))
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Return the open file diagnostic version when it matches a revision.
    pub(crate) fn open_file_version_in_revision(
        &self,
        repository: &Repository,
        revision: Revision,
        file_id: FileId,
        path: &Path,
    ) -> Result<Option<i32>, Error> {
        // missing open files have no client version
        let Some(file) = self.open_state(path) else {
            return Ok(None);
        };

        // only advertise a version when revision content matches open text
        let Some(content_id) = repository.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        if content_id != file.content_id {
            return Ok(None);
        }

        Ok(Some(file.version))
    }
}
