use std::path::PathBuf;

use destack_source::{Diagnostic, FileId, FileType, ModuleId, Uri};
use destack_workspace::InvalidationPlan;

/// Workspace handle identifier used by the local service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceHandleId(pub u64);

/// Reason for a workspace rescan request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RescanReason {
    /// Requested during startup synchronization.
    Startup,
    /// Requested after a watcher overflow.
    Overflow,
    /// Requested manually.
    Manual,
    /// Requested after watch updates.
    Update,
}

/// Message severity for local workspace records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceMessageKind {
    /// Informational message.
    Info,
    /// Warning message.
    Warning,
    /// Error message.
    Error,
}

/// Message payload emitted by the local workspace service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMessage {
    /// Message severity.
    pub kind: WorkspaceMessageKind,
    /// Stable message code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Snapshot for updated files.
#[derive(Debug, Clone, PartialEq)]
pub struct FileSnapshot {
    /// File id in the registry.
    pub id: FileId,
    /// File name.
    pub name: String,
    /// File uri.
    pub uri: Uri,
    /// Optional file path.
    pub path: Option<PathBuf>,
    /// File type.
    pub file_type: FileType,
    /// Optional text content.
    pub content: Option<String>,
}

/// Update record emitted by the local workspace service.
#[derive(Debug, Clone)]
pub struct WorkspaceUpdateRecord {
    /// Updated module id when known.
    pub module_id: Option<ModuleId>,
    /// Updated file id.
    pub file_id: FileId,
    /// Updated file snapshot.
    pub file: FileSnapshot,
    /// Invalidation summary.
    pub invalidation: InvalidationPlan,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result of applying local workspace service updates.
#[derive(Debug, Default)]
pub struct WorkspaceServiceResult {
    /// Update records produced by the operation.
    pub updates: Vec<WorkspaceUpdateRecord>,
    /// Message records produced by the operation.
    pub messages: Vec<WorkspaceMessage>,
}

/// Outcome of an explicit analyze request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzeOutcome {
    /// Whether semantic query state is ready for the analyzed module.
    /// Semantic query state means the module has both AST and profile DIR available.
    pub semantic_query_ready: bool,
    /// Optional detail when semantic query state is not ready.
    pub detail: Option<String>,
}
