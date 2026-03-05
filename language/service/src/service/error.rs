use std::path::PathBuf;

use crate::query::{QueryExecutionMode, QueryMethodId};
use destack_source::FileId;

use super::WorkspaceHandleId;

/// Errors produced by workspace service operations.
#[derive(Debug)]
pub enum LanguageServiceError {
    /// The workspace handle was not found.
    UnknownWorkspaceHandle {
        /// The unknown handle id.
        handle: WorkspaceHandleId,
    },
    /// The workspace handle was missing after opening a root.
    WorkspaceHandleMissingAfterOpen {
        /// The root path that failed to map.
        root: PathBuf,
    },
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
    /// Invalidation failed for a path.
    InvalidatePathFailed {
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
    /// The path is outside all opened workspace roots.
    PathNotInWorkspace {
        /// The path that failed workspace routing.
        path: PathBuf,
    },
    /// The query expected revision is missing for mutating requests.
    MissingExpectedRevision,
    /// The read query path received an unexpected revision precondition.
    UnexpectedExpectedRevisionOnRead {
        /// The unexpected revision carried on the request.
        expected_revision: u64,
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
    /// The query read path is blocked by an active workspace mutation.
    QueryBusy {
        /// The workspace handle that is currently mutating.
        handle: WorkspaceHandleId,
    },
    /// The query expected revision does not match the current workspace revision.
    StaleRevision {
        /// The caller expected revision.
        expected: u64,
        /// The current workspace revision.
        current: u64,
    },
    /// The semantic revision entry is missing for a workspace root.
    RevisionNotTracked {
        /// The workspace root missing revision state.
        root: PathBuf,
    },
    /// Semantic query state is not ready.
    SemanticQueryNotReady {
        /// The failure detail.
        detail: String,
    },
    /// Analyze operation failed.
    AnalyzeFailed {
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
            LanguageServiceError::UnknownWorkspaceHandle { handle } => {
                write!(formatter, "unknown workspace handle: {handle:?}")
            }
            LanguageServiceError::WorkspaceHandleMissingAfterOpen { root } => {
                write!(
                    formatter,
                    "workspace handle missing after open: {}",
                    root.display()
                )
            }
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
            LanguageServiceError::InvalidatePathFailed { path, detail } => {
                write!(
                    formatter,
                    "invalidate failed for {}: {detail}",
                    path.display()
                )
            }
            LanguageServiceError::FileNotTracked { path } => {
                write!(formatter, "file not tracked: {}", path.display())
            }
            LanguageServiceError::FileIdNotTracked { file_id } => {
                write!(formatter, "file id not tracked: {file_id:?}")
            }
            LanguageServiceError::PathNotInWorkspace { path } => {
                write!(
                    formatter,
                    "path is not in a workspace root: {}",
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
            LanguageServiceError::QueryBusy { handle } => {
                write!(formatter, "query busy for workspace handle: {handle:?}")
            }
            LanguageServiceError::StaleRevision { expected, current } => {
                write!(
                    formatter,
                    "stale query revision: expected {expected}, current {current}"
                )
            }
            LanguageServiceError::RevisionNotTracked { root } => {
                write!(
                    formatter,
                    "revision is not tracked for root: {}",
                    root.display()
                )
            }
            LanguageServiceError::SemanticQueryNotReady { detail } => {
                write!(formatter, "semantic query state is not ready: {detail}")
            }
            LanguageServiceError::AnalyzeFailed { detail } => {
                write!(formatter, "analyze failed: {detail}")
            }
            LanguageServiceError::Internal { detail } => {
                write!(formatter, "workspace service internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for LanguageServiceError {}
