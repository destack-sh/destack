use std::path::Path;
use std::sync::Arc;

use destack_source::{File, FileId, ModuleId, Uri};

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

        // package manifest
        if file_name == "package.json" {
            Self::Package
        }
        // destack manifest
        else if file_name == "destack.json" {
            Self::Destack
        }
        // typescript config
        else if file_name.starts_with("tsconfig") && file_name.ends_with(".json") {
            Self::TsConfig
        }
        // ordinary source
        else {
            Self::Unknown
        }
    }

    /// Return true when this kind is one config change.
    pub(crate) fn is_config_change(&self) -> bool {
        matches!(self, Self::Package | Self::Destack | Self::TsConfig)
    }
}

/// File update emitted by one live session.
#[derive(Debug, Clone)]
pub struct FileUpdate {
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
    /// Updated file id.
    pub file_id: FileId,
    /// Client-facing uri for this update.
    pub uri: Uri,
    /// Updated source file.
    pub file: Arc<File>,
    /// The coarse change kind for this file.
    pub kind: FileChangeKind,
}

/// Client-facing state for one open file.
#[derive(Debug, Clone)]
pub struct OpenFile {
    /// Client-facing uri for this open file.
    pub uri: Uri,
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
