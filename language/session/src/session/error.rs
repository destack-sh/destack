use std::path::PathBuf;

use destack_source::{FileId, ModuleId};
use destack_workspace::RepositoryError;

/// Errors produced by live session operations.
#[derive(Debug)]
pub enum SessionError {
    /// Path resolution failed.
    ResolvePathFailed {
        /// The path that failed.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// Semantic update failed for a path.
    UpdatePathFailed {
        /// The path that failed.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// Reading a path failed.
    ReadPathFailed {
        /// The path that failed.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// The file id is not tracked.
    FileIdNotTracked {
        /// The missing file id.
        file_id: FileId,
    },
    /// The module id is not tracked.
    ModuleIdNotTracked {
        /// The missing module id.
        module_id: ModuleId,
    },
    /// The incoming open-file version is not newer than the tracked version.
    StaleOpenFileVersion {
        /// The tracked file path.
        path: PathBuf,
        /// The incoming client version.
        incoming: i32,
        /// The current tracked client version.
        current: i32,
    },
    /// Repository work failed inside the session.
    Repository(RepositoryError),
    /// Internal session failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::ResolvePathFailed { path, detail } => {
                write!(formatter, "resolve failed for {}: {detail}", path.display())
            }
            SessionError::UpdatePathFailed { path, detail } => {
                write!(formatter, "update failed for {}: {detail}", path.display())
            }
            SessionError::ReadPathFailed { path, detail } => {
                write!(formatter, "read failed for {}: {detail}", path.display())
            }
            SessionError::FileIdNotTracked { file_id } => {
                write!(formatter, "file id not tracked: {file_id:?}")
            }
            SessionError::ModuleIdNotTracked { module_id } => {
                write!(formatter, "module id not tracked: {module_id:?}")
            }
            SessionError::StaleOpenFileVersion {
                path,
                incoming,
                current,
            } => {
                write!(
                    formatter,
                    "stale open file version for {}: incoming {incoming}, current {current}",
                    path.display()
                )
            }
            SessionError::Repository(error) => {
                write!(formatter, "session repository error: {error}")
            }
            SessionError::Internal { detail } => {
                write!(formatter, "session internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SessionError::Repository(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RepositoryError> for SessionError {
    fn from(error: RepositoryError) -> Self {
        SessionError::Repository(error)
    }
}
