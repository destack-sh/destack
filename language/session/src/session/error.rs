use tspp_artifact::{ArtifactFailure, ArtifactKey, DiagnosticError};
use tspp_repository::{Execution, RepositoryError};
use tspp_source::{FileId, ModuleId};

/// Errors produced by artifact session operations.
#[derive(Debug, Clone)]
pub enum SessionError {
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
    /// The worker count is not usable.
    InvalidWorkerCount {
        /// The invalid worker count.
        worker_count: usize,
    },
    /// The repository and artifact executor support different execution modes.
    ExecutionMismatch {
        /// Execution required by the repository host.
        repository: Execution,
        /// Execution provided by the artifact executor.
        executor: Execution,
    },
    /// Artifact provisioning was cancelled.
    Cancelled,
    /// Artifact provision reached a failed terminal outcome.
    ArtifactFailed {
        /// The failed artifact key.
        key: ArtifactKey,
        /// The artifact failure.
        failure: Box<ArtifactFailure>,
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
            SessionError::FileNotTracked { file_id } => {
                write!(formatter, "file not tracked: {file_id:?}")
            }
            SessionError::ModuleNotTracked { module_id } => {
                write!(formatter, "module not tracked: {module_id:?}")
            }
            SessionError::InvalidWorkerCount { worker_count } => {
                write!(formatter, "invalid artifact worker count: {worker_count}")
            }
            SessionError::ExecutionMismatch {
                repository,
                executor,
            } => {
                write!(
                    formatter,
                    "repository requires {repository:?} execution, artifact executor provides \
                     {executor:?} execution"
                )
            }
            SessionError::Cancelled => {
                write!(formatter, "artifact provisioning was cancelled")
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
