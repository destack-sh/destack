use std::path::PathBuf;

use destack_artifact::{ArtifactFailure, ArtifactKey, DiagnosticError};
use destack_source::{FileId, ModuleId, PackageId};
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
    /// The worker limit is not usable.
    InvalidWorkerLimit {
        /// The invalid worker limit.
        worker_limit: usize,
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
            SessionError::FileNotTracked { file_id } => {
                write!(formatter, "file not tracked: {file_id:?}")
            }
            SessionError::ModuleNotTracked { module_id } => {
                write!(formatter, "module not tracked: {module_id:?}")
            }
            SessionError::PackageNotTracked { package_id } => {
                write!(formatter, "package not tracked: {package_id:?}")
            }
            SessionError::InvalidWorkerLimit { worker_limit } => {
                write!(formatter, "invalid session worker limit: {worker_limit}")
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

impl From<DiagnosticError> for SessionError {
    fn from(error: DiagnosticError) -> Self {
        SessionError::Internal {
            detail: format!("failed to finalize provider diagnostic: {error}"),
        }
    }
}
