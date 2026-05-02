use std::path::Path;
use std::sync::Arc;

use destack_source::{File, FileId, ModuleId, Uri};

/// One explicit file-content update applied through a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    /// Replace file content with text.
    Text { content: String },
    /// Replace file content with raw bytes.
    Bytes { content: Vec<u8> },
    /// Remove the file from the revision.
    Removed,
}

/// One coarse kind for a file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileUpdateKind {
    /// One unknown or ordinary source change.
    Unknown,
    /// One `package.json` change.
    Package,
    /// One `destack.json` change.
    Destack,
    /// One `tsconfig*.json` change.
    TsConfig,
}

impl FileUpdateKind {
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
pub enum FileUpdate {
    /// A file that exists in the committed revision.
    Updated {
        /// Updated module id when known.
        module_id: Option<ModuleId>,
        /// Updated file id.
        file_id: FileId,
        /// Client-facing uri for this update.
        uri: Uri,
        /// Updated source file.
        file: Arc<File>,
        /// The coarse change kind for this file.
        kind: FileUpdateKind,
    },
    /// A file removed from the committed revision.
    Removed {
        /// Removed module id when known from the previous revision.
        module_id: Option<ModuleId>,
        /// Removed file id.
        file_id: FileId,
        /// Client-facing uri for this update.
        uri: Uri,
        /// The coarse change kind for this file.
        kind: FileUpdateKind,
    },
}

impl FileUpdate {
    /// Return the updated module id when known.
    pub fn module_id(&self) -> Option<ModuleId> {
        match self {
            Self::Updated { module_id, .. } | Self::Removed { module_id, .. } => *module_id,
        }
    }

    /// Return the updated file id.
    pub fn file_id(&self) -> FileId {
        match self {
            Self::Updated { file_id, .. } | Self::Removed { file_id, .. } => *file_id,
        }
    }

    /// Return the client-facing uri for this update.
    pub fn uri(&self) -> &Uri {
        match self {
            Self::Updated { uri, .. } | Self::Removed { uri, .. } => uri,
        }
    }

    /// Return the updated file when it still exists.
    pub fn file(&self) -> Option<&Arc<File>> {
        match self {
            Self::Updated { file, .. } => Some(file),
            Self::Removed { .. } => None,
        }
    }

    /// Return the coarse change kind for this file.
    pub fn kind(&self) -> FileUpdateKind {
        match self {
            Self::Updated { kind, .. } | Self::Removed { kind, .. } => *kind,
        }
    }

    /// Return whether this update removed a file.
    pub fn is_removed(&self) -> bool {
        matches!(self, Self::Removed { .. })
    }
}
