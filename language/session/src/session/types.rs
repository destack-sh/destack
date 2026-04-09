use std::path::PathBuf;

use destack_source::{Diagnostic, File, FileContent, FileId, FileType, ModuleId, Uri};
use destack_workspace::{Repository, Revision};

use crate::{Session, SessionError};

/// One explicit file-content update applied through a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileMutation {
    /// Replace file content with text.
    Text { content: String },
    /// Replace file content with raw bytes.
    Bytes { content: Vec<u8> },
    /// Remove the file from the revision.
    Removed,
}

/// One coarse kind for a file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileChangeKind {
    /// One unknown or ordinary source change.
    Unknown,
    /// One `package.json` change.
    Package,
    /// One `destack.json` change.
    Destack,
    /// One `tsconfig*.json` change.
    TsConfig,
}

impl FileChangeKind {
    /// Return the coarse change kind for one path.
    pub(crate) fn for_path(path: &std::path::Path) -> Self {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Self::Unknown;
        };

        if file_name == "package.json" {
            return Self::Package;
        }

        if file_name == "destack.json" {
            return Self::Destack;
        }

        if file_name.starts_with("tsconfig") && file_name.ends_with(".json") {
            return Self::TsConfig;
        }

        Self::Unknown
    }

    /// Return true when this kind is one config change.
    pub(crate) fn is_config_change(&self) -> bool {
        matches!(self, Self::Package | Self::Destack | Self::TsConfig)
    }
}

/// Snapshot for updated files.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSnapshot {
    /// File id in the registry.
    pub id: FileId,
    /// File name.
    pub name: String,
    /// File uri.
    pub uri: Uri,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// File type.
    pub file_type: FileType,
    /// Optional text content.
    pub content: Option<String>,
}

/// File update emitted by one live session.
#[derive(Debug, Clone)]
pub struct FileUpdate {
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
    /// Updated file id.
    pub file_id: FileId,
    /// Publish uri for this update.
    pub publish_uri: Uri,
    /// Publish version for this update when it comes from one tracked open file.
    pub publish_version: Option<i32>,
    /// Updated file snapshot.
    pub file: FileSnapshot,
    /// The coarse change kind for this file.
    pub kind: FileChangeKind,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// One file change tracked through one session update.
#[derive(Debug, Clone)]
pub(crate) struct FileChange {
    /// The changed module id when known.
    pub(crate) module_id: Option<ModuleId>,
    /// The file id for this change.
    pub(crate) file_id: FileId,
    /// The coarse change kind for this file.
    pub(crate) kind: FileChangeKind,
}

/// Build a file snapshot for a repository file id.
pub(crate) fn file_snapshot_for_id(
    repository: &Repository,
    revision: Revision,
    file_id: FileId,
) -> Result<FileSnapshot, SessionError> {
    let file = repository
        .file(revision, file_id)
        .map_err(SessionError::from)?
        .ok_or(SessionError::FileIdNotTracked { file_id })?;

    Ok(file_snapshot_from_file(&file))
}

/// Build a file snapshot payload.
pub(crate) fn file_snapshot_from_file(file: &File) -> FileSnapshot {
    let content = match file.content.payload() {
        FileContent::Text { content } => Some(content.clone()),
        FileContent::Binary { .. } => None,
    };

    FileSnapshot {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    }
}

impl Session {
    /// Build one publish identity for a file snapshot.
    fn publish_identity(
        &self,
        file: &FileSnapshot,
        preferred_uri: Option<&Uri>,
    ) -> (Uri, Option<i32>) {
        let open_file_identity = file
            .path
            .as_ref()
            .and_then(|path| self.open_file_identity_for_path(path));

        let publish_uri = open_file_identity
            .as_ref()
            .map(|(uri, _)| uri.clone())
            .or_else(|| preferred_uri.cloned())
            .or_else(|| file.path.as_ref().map(Uri::from_file_path))
            .unwrap_or_else(|| file.uri.clone());
        let publish_version = open_file_identity.as_ref().map(|(_, version)| *version);

        (publish_uri, publish_version)
    }

    /// Build public file updates for one file change batch.
    pub(crate) fn file_updates(
        &self,
        revision: Revision,
        file_changes: Vec<FileChange>,
        mut diagnostics_by_file: std::collections::HashMap<FileId, Vec<Diagnostic>>,
        preferred_uri: Option<&Uri>,
    ) -> Result<Vec<FileUpdate>, SessionError> {
        let mut updates = Vec::new();

        for file_change in file_changes {
            let file = file_snapshot_for_id(&self.repository, revision, file_change.file_id)?;
            let diagnostics = diagnostics_by_file
                .remove(&file_change.file_id)
                .unwrap_or_default();
            let (publish_uri, publish_version) = self.publish_identity(&file, preferred_uri);

            updates.push(FileUpdate {
                module_id: file_change.module_id,
                file_id: file_change.file_id,
                publish_uri,
                publish_version,
                file,
                kind: file_change.kind,
                diagnostics,
            });
        }

        Ok(updates)
    }
}
