use std::path::PathBuf;

use destack_query::{QueryExecutionMode, QueryMethodId};
use destack_session::SessionError;
use destack_source::{FileId, ModuleId};
use destack_workspace::{RepositoryError, Revision};

/// Errors produced by workspace service operations.
#[derive(Debug)]
pub enum LanguageServiceError {
    /// Cache clearing failed for a path.
    CacheClearFailed {
        /// Cache directory path.
        path: PathBuf,
        /// The failure detail.
        detail: String,
    },
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
    /// The file path is not tracked.
    FileNotTracked {
        /// The missing file path.
        path: PathBuf,
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
    /// The path is outside all opened workspace roots.
    PathNotInWorkspace {
        /// The path that failed workspace routing.
        path: PathBuf,
    },
    /// The incoming document version is not newer than the tracked version.
    StaleDocumentVersion {
        /// The tracked document path.
        path: PathBuf,
        /// The incoming client document version.
        incoming: i32,
        /// The current tracked client document version.
        current: i32,
    },
    /// The query expected revision is missing for mutating requests.
    MissingExpectedRevision,
    /// The read query path received an unexpected revision precondition.
    UnexpectedExpectedRevisionOnRead {
        /// The unexpected revision carried on the request.
        expected_revision: Revision,
    },
    /// The query execution mode does not match the called API.
    QueryExecutionModeMismatch {
        /// The query method identifier.
        method: QueryMethodId,
        /// The expected query execution mode.
        expected: QueryExecutionMode,
        /// The actual query execution mode.
        actual: QueryExecutionMode,
    },
    /// The query expected revision does not match the current semantic revision.
    StaleRevision {
        /// The caller expected revision.
        expected: Revision,
        /// The current semantic revision.
        current: Revision,
    },
    /// Repository work failed inside the service.
    Repository {
        /// The failure detail.
        detail: String,
    },
    /// The semantic revision entry is missing for a workspace root.
    RevisionNotTracked {
        /// The workspace root missing revision state.
        root: PathBuf,
    },
    /// Query artifacts are not ready.
    QueryNotReady {
        /// The failure detail.
        detail: String,
    },
    /// Internal workspace service failure.
    Internal {
        /// The failure detail.
        detail: String,
    },
}

impl std::fmt::Display for LanguageServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LanguageServiceError::CacheClearFailed { path, detail } => {
                write!(
                    formatter,
                    "cache clear failed at {}: {detail}",
                    path.display()
                )
            }
            LanguageServiceError::ResolvePathFailed { path, detail } => {
                write!(formatter, "resolve failed for {}: {detail}", path.display())
            }
            LanguageServiceError::UpdatePathFailed { path, detail } => {
                write!(formatter, "update failed for {}: {detail}", path.display())
            }
            LanguageServiceError::ReadPathFailed { path, detail } => {
                write!(formatter, "read failed for {}: {detail}", path.display())
            }
            LanguageServiceError::FileNotTracked { path } => {
                write!(formatter, "file not tracked: {}", path.display())
            }
            LanguageServiceError::FileIdNotTracked { file_id } => {
                write!(formatter, "file id not tracked: {file_id:?}")
            }
            LanguageServiceError::ModuleIdNotTracked { module_id } => {
                write!(formatter, "module id not tracked: {module_id:?}")
            }
            LanguageServiceError::PathNotInWorkspace { path } => {
                write!(
                    formatter,
                    "path is not in a workspace root: {}",
                    path.display()
                )
            }
            LanguageServiceError::StaleDocumentVersion {
                path,
                incoming,
                current,
            } => {
                write!(
                    formatter,
                    "stale document version for {}: incoming {incoming}, current {current}",
                    path.display()
                )
            }
            LanguageServiceError::MissingExpectedRevision => {
                write!(formatter, "missing expected revision for mutating query")
            }
            LanguageServiceError::UnexpectedExpectedRevisionOnRead { expected_revision } => {
                write!(
                    formatter,
                    "read query must not carry expected revision: {expected_revision}"
                )
            }
            LanguageServiceError::QueryExecutionModeMismatch {
                method,
                expected,
                actual,
            } => {
                write!(
                    formatter,
                    "query execution mode mismatch for {method:?}: expected {expected:?}, actual {actual:?}"
                )
            }
            LanguageServiceError::StaleRevision { expected, current } => {
                write!(
                    formatter,
                    "stale query revision: expected {expected}, current {current}"
                )
            }
            LanguageServiceError::Repository { detail } => {
                write!(formatter, "workspace service repository error: {detail}")
            }
            LanguageServiceError::RevisionNotTracked { root } => {
                write!(
                    formatter,
                    "revision is not tracked for root: {}",
                    root.display()
                )
            }
            LanguageServiceError::QueryNotReady { detail } => {
                write!(formatter, "query artifacts are not ready: {detail}")
            }
            LanguageServiceError::Internal { detail } => {
                write!(formatter, "workspace service internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for LanguageServiceError {}

impl From<RepositoryError> for LanguageServiceError {
    fn from(error: RepositoryError) -> Self {
        LanguageServiceError::Repository {
            detail: error.to_string(),
        }
    }
}

impl From<SessionError> for LanguageServiceError {
    fn from(error: SessionError) -> Self {
        match error {
            SessionError::ResolvePathFailed { path, detail } => {
                LanguageServiceError::ResolvePathFailed { path, detail }
            }
            SessionError::UpdatePathFailed { path, detail } => {
                LanguageServiceError::UpdatePathFailed { path, detail }
            }
            SessionError::ReadPathFailed { path, detail } => {
                LanguageServiceError::ReadPathFailed { path, detail }
            }
            SessionError::FileIdNotTracked { file_id } => {
                LanguageServiceError::FileIdNotTracked { file_id }
            }
            SessionError::ModuleIdNotTracked { module_id } => {
                LanguageServiceError::ModuleIdNotTracked { module_id }
            }
            SessionError::StaleOpenFileVersion {
                path,
                incoming,
                current,
            } => LanguageServiceError::StaleDocumentVersion {
                path,
                incoming,
                current,
            },
            SessionError::Repository { detail } => LanguageServiceError::Repository { detail },
            SessionError::Internal { detail } => LanguageServiceError::Internal { detail },
        }
    }
}
