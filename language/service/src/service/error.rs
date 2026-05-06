use std::path::PathBuf;

use destack_query::{QueryExecutionMode, QueryMethodId};
use destack_session::SessionError;
use destack_workspace::{RepositoryError, Revision};

/// Errors produced by language service operations.
#[derive(Debug)]
pub enum LanguageServiceError {
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
    /// The query execution mode does not match the called API.
    QueryModeMismatch {
        /// The query method identifier.
        method: QueryMethodId,
        /// The expected query execution mode.
        expected: QueryExecutionMode,
        /// The actual query execution mode.
        actual: QueryExecutionMode,
    },
    /// The query expected revision does not match the current revision.
    StaleRevision {
        /// The caller expected revision.
        expected: Revision,
        /// The current revision.
        current: Revision,
    },
    /// Query artifacts are not ready.
    QueryNotReady {
        /// The failure detail.
        detail: String,
    },
    /// Repository work failed inside the service.
    Repository(RepositoryError),
    /// Session work failed inside the service.
    Session(Box<SessionError>),
    /// Filesystem work failed inside the service.
    Io {
        /// The path that failed.
        path: PathBuf,
        /// The filesystem failure.
        source: std::io::Error,
    },
    /// Internal language service failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for LanguageServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LanguageServiceError::PathNotInRoot { path } => {
                write!(formatter, "path is outside every root: {}", path.display())
            }
            LanguageServiceError::FileMissing { path } => {
                write!(formatter, "file is missing: {}", path.display())
            }
            LanguageServiceError::StaleOpenFile {
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
            LanguageServiceError::InvalidTextChange { path, detail } => {
                write!(
                    formatter,
                    "invalid text change for {}: {detail}",
                    path.display()
                )
            }
            LanguageServiceError::QueryModeMismatch {
                method,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "query mode mismatch for {method:?}: expected {expected:?}, actual {actual:?}"
                )
            }
            LanguageServiceError::StaleRevision { expected, current } => {
                write!(
                    formatter,
                    "stale query revision: expected {expected}, current {current}"
                )
            }
            LanguageServiceError::QueryNotReady { detail } => {
                write!(formatter, "query indexes are not ready: {detail}")
            }
            LanguageServiceError::Repository(error) => {
                write!(formatter, "repository error: {error}")
            }
            LanguageServiceError::Session(error) => {
                write!(formatter, "session error: {error}")
            }
            LanguageServiceError::Io { path, source } => {
                write!(
                    formatter,
                    "filesystem error at {}: {source}",
                    path.display()
                )
            }
            LanguageServiceError::Internal { detail } => {
                write!(formatter, "language service internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for LanguageServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            LanguageServiceError::Repository(error) => Some(error),
            LanguageServiceError::Session(error) => Some(error),
            LanguageServiceError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<RepositoryError> for LanguageServiceError {
    fn from(error: RepositoryError) -> Self {
        LanguageServiceError::Repository(error)
    }
}

impl From<SessionError> for LanguageServiceError {
    fn from(error: SessionError) -> Self {
        LanguageServiceError::Session(Box::new(error))
    }
}
