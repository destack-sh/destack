use std::path::PathBuf;

use destack_workspace::Error;

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
    /// Workspace operation failed.
    Workspace {
        /// The typed workspace error.
        error: Error,
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
            DaemonError::Workspace { error } => {
                write!(f, "workspace error: {error}")
            }
        }
    }
}

impl std::error::Error for DaemonError {}

impl From<Error> for DaemonError {
    fn from(error: Error) -> Self {
        Self::Workspace { error }
    }
}
