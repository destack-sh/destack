use std::path::PathBuf;

/// Errors produced by external source operations.
#[derive(Debug)]
pub enum SourceError {
    /// The source path is not valid for this source.
    InvalidPath {
        /// The invalid path.
        path: PathBuf,
        /// The failure message.
        message: String,
    },
    /// One source read or listing operation failed.
    ReadFailed {
        /// The failed operation.
        operation: &'static str,
        /// The path used by the operation.
        path: PathBuf,
        /// The failure message.
        message: String,
    },
    /// One source write operation failed.
    WriteFailed {
        /// The failed operation.
        operation: &'static str,
        /// The path used by the operation.
        path: PathBuf,
        /// The failure message.
        message: String,
    },
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath { path, message } => {
                write!(
                    formatter,
                    "invalid source path '{}': {message}",
                    path.display()
                )
            }
            Self::ReadFailed {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "source read failed during {operation} for '{}': {message}",
                    path.display()
                )
            }
            Self::WriteFailed {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "source write failed during {operation} for '{}': {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for SourceError {}
