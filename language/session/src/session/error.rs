use std::path::PathBuf;

use destack_artifact::{ArtifactFailure, ArtifactKey, DiagnosticError};
use destack_source::{FileId, ModuleId, PackageId};
use destack_workspace::RepositoryError;

use crate::SourceError;

/// Errors produced by live session operations.
#[derive(Debug)]
pub enum SessionError {
    /// A filesystem path cannot be loaded as a module.
    ModulePathNotLoadable {
        /// The path that could not be loaded.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
    /// The file is not tracked.
    FileNotTracked {
        /// The missing file id.
        file_id: FileId,
    },
    /// The module is not tracked.
    ModuleNotTracked {
        /// The missing module id.
        module_id: ModuleId,
    },
    /// The package is not tracked.
    PackageNotTracked {
        /// The missing package id.
        package_id: PackageId,
    },
    /// The worker count is not usable.
    InvalidWorkerCount {
        /// The invalid worker count.
        worker_count: usize,
    },
    /// Artifact provision reached a failed terminal outcome.
    ArtifactFailed {
        /// The failed artifact key.
        key: ArtifactKey,
        /// The artifact failure.
        failure: ArtifactFailure,
    },
    /// Repository work failed inside the session.
    Repository(RepositoryError),
    /// External source work failed inside the session.
    Source(SourceError),
    /// Internal session failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::ModulePathNotLoadable { path, detail } => {
                write!(
                    formatter,
                    "module path is not loadable for {}: {detail}",
                    path.display()
                )
            }
            SessionError::FileNotTracked { file_id } => {
                write!(formatter, "file not tracked: {file_id:?}")
            }
            SessionError::ModuleNotTracked { module_id } => {
                write!(formatter, "module not tracked: {module_id:?}")
            }
            SessionError::PackageNotTracked { package_id } => {
                write!(formatter, "package not tracked: {package_id:?}")
            }
            SessionError::InvalidWorkerCount { worker_count } => {
                write!(formatter, "invalid session worker count: {worker_count}")
            }
            SessionError::ArtifactFailed { key, failure } => {
                write!(
                    formatter,
                    "artifact failed while providing {key:?}: {failure:?}"
                )
            }
            SessionError::Repository(error) => {
                write!(formatter, "session repository error: {error}")
            }
            SessionError::Source(error) => {
                write!(formatter, "session source error: {error}")
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
            SessionError::Source(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RepositoryError> for SessionError {
    fn from(error: RepositoryError) -> Self {
        SessionError::Repository(error)
    }
}

impl From<SourceError> for SessionError {
    fn from(error: SourceError) -> Self {
        SessionError::Source(error)
    }
}

impl From<DiagnosticError> for SessionError {
    fn from(error: DiagnosticError) -> Self {
        SessionError::Internal {
            detail: format!("failed to finalize provider diagnostic: {error}"),
        }
    }
}
