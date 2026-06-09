use std::path::PathBuf;

use destack_repository::{RepositoryError, Revision};
use destack_session::SessionError;

/// Errors produced by workspace operations.
#[derive(Debug)]
pub enum Error {
    /// A path is outside every opened root.
    PathNotInRoot {
        /// The path that failed root routing.
        path: PathBuf,
    },
    /// A requested file is missing from the current revision.
    FileMissing {
        /// The missing file path.
        path: PathBuf,
    },
    /// The incoming open file version is not newer than the tracked version.
    StaleOpenFile {
        /// The open file path.
        path: PathBuf,
        /// The incoming client file version.
        incoming: i32,
        /// The current tracked client file version.
        current: i32,
    },
    /// The requested text change is invalid.
    InvalidTextChange {
        /// The changed file path.
        path: PathBuf,
        /// The validation failure detail.
        detail: String,
    },
    /// The requested edit is not valid for this workspace operation.
    InvalidEdit {
        /// The validation failure detail.
        detail: String,
    },
    /// The query expected revision does not match the current revision.
    StaleRevision {
        /// The caller expected revision.
        expected: Revision,
        /// The current revision.
        current: Revision,
    },
    /// Repository work failed inside the workspace.
    Repository(RepositoryError),
    /// Session work failed inside the workspace.
    Session(Box<SessionError>),
    /// Filesystem work failed inside the workspace.
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The filesystem failure.
        source: std::io::Error,
    },
    /// Internal workspace failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PathNotInRoot { path } => {
                write!(formatter, "path is outside every root: {}", path.display())
            }
            Error::FileMissing { path } => {
                write!(formatter, "file is missing: {}", path.display())
            }
            Error::StaleOpenFile {
                path,
                incoming,
                current,
            } => {
                write!(
                    formatter,
                    "stale open file for {}: incoming {incoming}, current {current}",
                    path.display()
                )
            }
            Error::InvalidTextChange { path, detail } => {
                write!(
                    formatter,
                    "invalid text change for {}: {detail}",
                    path.display()
                )
            }
            Error::InvalidEdit { detail } => {
                write!(formatter, "invalid edit: {detail}")
            }
            Error::StaleRevision { expected, current } => {
                write!(
                    formatter,
                    "stale query revision: expected {expected}, current {current}"
                )
            }
            Error::Repository(error) => {
                write!(formatter, "repository error: {error}")
            }
            Error::Session(error) => {
                write!(formatter, "session error: {error}")
            }
            Error::Io { path, source } => {
                write!(
                    formatter,
                    "filesystem error at {}: {source}",
                    path.display()
                )
            }
            Error::Internal { detail } => {
                write!(formatter, "workspace internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Repository(error) => Some(error),
            Error::Session(error) => Some(error),
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<RepositoryError> for Error {
    fn from(error: RepositoryError) -> Self {
        Error::Repository(error)
    }
}

impl From<SessionError> for Error {
    fn from(error: SessionError) -> Self {
        Error::Session(Box::new(error))
    }
}
