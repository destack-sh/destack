use destack_source::FileId;

/// One workspace construction error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceError {
    /// The config file id.
    pub file_id: FileId,
    /// The config path.
    pub path: String,
    /// The error message.
    pub message: String,
}

impl WorkspaceError {
    /// Create one workspace construction error.
    pub(crate) fn new(file_id: FileId, path: String, message: String) -> Self {
        Self {
            file_id,
            path,
            message,
        }
    }
}
