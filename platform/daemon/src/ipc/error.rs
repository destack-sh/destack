use std::io;

/// Errors for daemon ipc operations.
#[derive(Debug)]
pub enum DaemonIpcError {
    /// Io error.
    Io(io::Error),
    /// Listener would block.
    WouldBlock,
    /// Unsupported platform.
    Unsupported,
}

impl std::fmt::Display for DaemonIpcError {
    /// Format the daemon ipc error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DaemonIpcError::Io(error) => write!(f, "daemon ipc error: {error}"),
            DaemonIpcError::WouldBlock => write!(f, "daemon ipc would block"),
            DaemonIpcError::Unsupported => write!(f, "daemon ipc is not supported"),
        }
    }
}

impl std::error::Error for DaemonIpcError {}

impl From<io::Error> for DaemonIpcError {
    /// Convert an io error to a daemon ipc error.
    fn from(error: io::Error) -> Self {
        DaemonIpcError::Io(error)
    }
}
