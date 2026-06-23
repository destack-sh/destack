use std::io;
use std::path::PathBuf;

/// Errors for workspace ipc operations.
#[derive(Debug)]
pub enum WorkspaceIpcError {
    /// Io error.
    Io(io::Error),
    /// Invalid ipc path.
    InvalidPath(PathBuf),
    /// Unsupported platform.
    Unsupported,
}

impl std::fmt::Display for WorkspaceIpcError {
    /// Format the workspace ipc error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceIpcError::Io(error) => write!(f, "workspace ipc error: {error}"),
            WorkspaceIpcError::InvalidPath(path) => {
                write!(f, "invalid workspace ipc path: {}", path.display())
            }
            WorkspaceIpcError::Unsupported => write!(f, "workspace ipc is not supported"),
        }
    }
}

impl std::error::Error for WorkspaceIpcError {}

impl From<io::Error> for WorkspaceIpcError {
    /// Convert an io error to a workspace ipc error.
    fn from(error: io::Error) -> Self {
        WorkspaceIpcError::Io(error)
    }
}
