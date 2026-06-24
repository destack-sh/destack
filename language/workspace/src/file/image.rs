use destack_serde::Reflect;
use std::path::{Path, PathBuf};

use destack_session::Change;
use destack_source::{Content, Diagnostic, File, FileId, FileType, ModuleId, TextChange, Uri};
use serde::{Deserialize, Serialize};

use crate::diagnostic::Error;

/// In-memory image for one updated file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileImage {
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

impl From<&File> for FileImage {
    /// Build a file image from one source file.
    fn from(file: &File) -> Self {
        let content = match file.content.payload() {
            Content::Text { content } => Some(content.clone()),
            Content::Binary { .. } => None,
        };

        Self {
            id: file.id,
            name: file.name.clone(),
            uri: file.uri.clone(),
            path: file.path.clone(),
            file_type: file.ty,
            content,
        }
    }
}

impl FileImage {
    /// Convert this image into a source file.
    pub fn into_file(self) -> Result<File, Error> {
        let Some(content) = self.content else {
            return Err(Error::Internal {
                detail: format!("file image is missing text content for {}", self.name),
            });
        };

        Ok(File::from_text(
            self.id,
            self.name,
            self.uri,
            self.path,
            self.file_type,
            content,
        ))
    }
}

/// One coarse kind for a workspace file update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum UpdateKind {
    /// One ordinary source change.
    Source,
    /// One `destack.json` change.
    Config,
}

impl UpdateKind {
    /// Return the coarse update kind for one path.
    pub(crate) fn for_path(path: &std::path::Path) -> Self {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Self::Source;
        };

        // destack manifest
        if file_name == "destack.json" {
            Self::Config
        }
        // ordinary source
        else {
            Self::Source
        }
    }
}

/// File update emitted by the workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileUpdate {
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
    /// Updated file id.
    pub file_id: FileId,
    /// Diagnostic uri for this update.
    pub diagnostic_uri: Uri,
    /// Protocol file version for diagnostics when the file is open.
    pub diagnostic_version: Option<i32>,
    /// Updated file image when the file still exists.
    pub file: Option<FileImage>,
    /// Whether this update removed the file.
    pub is_removed: bool,
    /// The coarse change kind for this file.
    pub kind: UpdateKind,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

impl From<Change> for FileUpdate {
    /// Project one session change into a workspace payload.
    fn from(update: Change) -> Self {
        match update {
            Change::Updated {
                module_id,
                file_id,
                uri,
                file,
            } => {
                let kind = file
                    .path
                    .as_deref()
                    .map(UpdateKind::for_path)
                    .unwrap_or(UpdateKind::Source);

                Self {
                    module_id,
                    file_id,
                    diagnostic_uri: uri,
                    diagnostic_version: None,
                    file: Some(FileImage::from(file.as_ref())),
                    is_removed: false,
                    kind,
                    diagnostics: Vec::new(),
                }
            }
            Change::Removed {
                module_id,
                file_id,
                uri,
            } => {
                let kind = uri
                    .to_path_buf()
                    .as_deref()
                    .map(UpdateKind::for_path)
                    .unwrap_or(UpdateKind::Source);

                Self {
                    module_id,
                    file_id,
                    diagnostic_uri: uri,
                    diagnostic_version: None,
                    file: None,
                    is_removed: true,
                    kind,
                    diagnostics: Vec::new(),
                }
            }
        }
    }
}

/// File operation applied through a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum FileOperation {
    /// Open editor text content.
    OpenText {
        /// Path being opened.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current text content.
        content: String,
    },
    /// Open editor binary content.
    OpenBytes {
        /// Path being opened.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Change editor text content.
    ChangeText {
        /// Path being changed.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current text content.
        content: String,
    },
    /// Change editor binary content.
    ChangeBytes {
        /// Path being changed.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Patch editor text content.
    PatchText {
        /// Path being patched.
        path: PathBuf,
        /// Editor document URI.
        uri: Uri,
        /// Editor document version.
        version: i32,
        /// Incremental text changes.
        changes: Vec<TextChange>,
    },
    /// Save editor text content.
    SaveText {
        /// Path being saved.
        path: PathBuf,
        /// Current text content.
        content: Option<String>,
    },
    /// Save editor binary content.
    SaveBytes {
        /// Path being saved.
        path: PathBuf,
        /// Current binary content.
        content: Option<Vec<u8>>,
    },
    /// Close editor overlay state and restore filesystem truth.
    Close {
        /// Path being closed.
        path: PathBuf,
    },
    /// Write text content to disk and workspace state.
    WriteText {
        /// Path being written.
        path: PathBuf,
        /// Current text content.
        content: String,
    },
    /// Write binary content to disk and workspace state.
    WriteBytes {
        /// Path being written.
        path: PathBuf,
        /// Current binary content.
        content: Vec<u8>,
    },
    /// Remove a file from disk and workspace state.
    Remove {
        /// Path being removed.
        path: PathBuf,
    },
    /// Move a file on disk and workspace state.
    Move {
        /// Source path.
        from: PathBuf,
        /// Destination path.
        to: PathBuf,
    },
}

impl FileOperation {
    /// Return the source path for this operation.
    pub fn path(&self) -> &Path {
        match self {
            Self::OpenText { path, .. }
            | Self::OpenBytes { path, .. }
            | Self::ChangeText { path, .. }
            | Self::ChangeBytes { path, .. }
            | Self::PatchText { path, .. }
            | Self::SaveText { path, .. }
            | Self::SaveBytes { path, .. }
            | Self::Close { path }
            | Self::WriteText { path, .. }
            | Self::WriteBytes { path, .. }
            | Self::Remove { path }
            | Self::Move { from: path, .. } => path,
        }
    }
}
