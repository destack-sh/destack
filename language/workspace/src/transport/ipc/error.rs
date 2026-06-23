use std::io;
use std::path::PathBuf;

/// Errors for workspace ipc operations.
#[derive(Debug)]
pub enum IpcError {
    /// Io error.
    Io(io::Error),
    /// Invalid ipc path.
    InvalidPath(PathBuf),
    /// Unsupported platform.
    Unsupported,
}

impl std::fmt::Display for IpcError {
    /// Format the workspace ipc error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpcError::Io(error) => write!(f, "workspace ipc error: {error}"),
            IpcError::InvalidPath(path) => {
                write!(f, "invalid workspace ipc path: {}", path.display())
            }
            IpcError::Unsupported => write!(f, "workspace ipc is not supported"),
        }
    }
}

impl std::error::Error for IpcError {}

impl From<io::Error> for IpcError {
    /// Convert an io error to a workspace ipc error.
    fn from(error: io::Error) -> Self {
        IpcError::Io(error)
    }
}
