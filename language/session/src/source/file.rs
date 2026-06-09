use std::sync::Arc;

use destack_source::{File, FileId, ModuleId, Uri};

/// Observed file change emitted by one live session.
#[derive(Debug, Clone)]
pub enum Change {
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
    },
    /// A file removed from the committed revision.
    Removed {
        /// Removed module id when known from the previous revision.
        module_id: Option<ModuleId>,
        /// Removed file id.
        file_id: FileId,
        /// Client-facing uri for this update.
        uri: Uri,
    },
}

impl Change {
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

    /// Return whether this update removed a file.
    pub fn is_removed(&self) -> bool {
        matches!(self, Self::Removed { .. })
    }
}
