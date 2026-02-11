use std::path::PathBuf;

use destack_source::FileId;

use super::WorkspaceHandleId;

/// Errors produced by workspace service operations.
#[derive(Debug)]
pub enum WorkspaceServiceError {
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

impl std::fmt::Display for WorkspaceServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceServiceError::UnknownWorkspaceHandle { handle } => {
                write!(formatter, "unknown workspace handle: {handle:?}")
            }
            WorkspaceServiceError::WorkspaceHandleMissingAfterOpen { root } => {
                write!(
                    formatter,
                    "workspace handle missing after open: {}",
                    root.display()
                )
            }
            WorkspaceServiceError::CacheClearFailed { path, detail } => {
                write!(
                    formatter,
                    "cache clear failed at {}: {detail}",
                    path.display()
                )
            }
            WorkspaceServiceError::ResolvePathFailed { path, detail } => {
                write!(formatter, "resolve failed for {}: {detail}", path.display())
            }
            WorkspaceServiceError::InvalidatePathFailed { path, detail } => {
                write!(
                    formatter,
                    "invalidate failed for {}: {detail}",
                    path.display()
                )
            }
            WorkspaceServiceError::FileNotTracked { path } => {
                write!(formatter, "file not tracked: {}", path.display())
            }
            WorkspaceServiceError::FileIdNotTracked { file_id } => {
                write!(formatter, "file id not tracked: {file_id:?}")
            }
            WorkspaceServiceError::SemanticQueryNotReady { detail } => {
                write!(formatter, "semantic query state is not ready: {detail}")
            }
            WorkspaceServiceError::AnalyzeFailed { detail } => {
                write!(formatter, "analyze failed: {detail}")
            }
            WorkspaceServiceError::Internal { detail } => {
                write!(formatter, "workspace service internal error: {detail}")
            }
        }
    }
}

impl std::error::Error for WorkspaceServiceError {}
