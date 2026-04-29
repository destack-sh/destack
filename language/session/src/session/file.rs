use std::path::{Path, PathBuf};

use destack_source::{Diagnostic, File, FileContent, FileId, FileType, ModuleId, Uri};

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
    pub(crate) fn for_path(path: &Path) -> Self {
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

/// Serializable image for one updated file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileUpdateImage {
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
    /// Diagnostic uri for this update.
    pub diagnostic_uri: Uri,
    /// Diagnostic version for this update when it comes from one tracked open file.
    pub diagnostic_version: Option<i32>,
    /// Updated file image.
    pub file: FileUpdateImage,
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

/// Build a file image payload.
pub(crate) fn file_update_image_from_file(file: &File) -> FileUpdateImage {
    let content = match file.content.payload() {
        FileContent::Text { content } => Some(content.clone()),
        FileContent::Binary { .. } => None,
    };

    FileUpdateImage {
        id: file.id,
        name: file.name.clone(),
        uri: file.uri.clone(),
        path: file.path.clone(),
        file_type: file.ty,
        content,
    }
}
