use std::path::PathBuf;

use destack_service::LanguageServiceError;

/// Errors produced while applying daemon updates.
#[derive(Debug)]
pub enum DaemonError {
    /// Workspace repository load failed.
    WorkspaceOpen {
        /// The workspace root.
        root: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// File system write failed.
    FileWrite {
        /// The file path that failed.
        path: PathBuf,
        /// The underlying write error.
        error: std::io::Error,
    },
    /// Language service operation failed.
    Service {
        /// The typed language service error.
        error: LanguageServiceError,
    },
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // format daemon errors
        match self {
            DaemonError::WorkspaceOpen { root, detail } => {
                write!(f, "failed to open workspace {}: {detail}", root.display())
            }
            DaemonError::FileWrite { path, error } => {
                write!(f, "failed to write file {}: {}", path.display(), error)
            }
            DaemonError::Service { error } => {
                write!(f, "service error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<LanguageServiceError> for DaemonError {
    fn from(error: LanguageServiceError) -> Self {
        Self::Service { error }
    }
}
