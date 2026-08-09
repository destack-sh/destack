use std::io;
use std::path::PathBuf;

/// Failure to open or accept a local RPC connection.
#[derive(Debug)]
pub enum IpcError {
    /// Underlying input or output failure.
    Io(io::Error),
    /// Invalid local connection path.
    InvalidPath(PathBuf),
}

impl std::fmt::Display for IpcError {
    /// Format this local connection failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "RPC IPC failed: {error}"),
            Self::InvalidPath(path) => {
                write!(formatter, "invalid RPC IPC path: {}", path.display())
            }
        }
    }
}

impl std::error::Error for IpcError {
    /// Return the underlying input or output failure when present.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidPath(_) => None,
        }
    }
}

impl From<io::Error> for IpcError {
    /// Convert an input or output failure.
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
