use std::path::PathBuf;
use std::sync::Arc;

use destack_source::{Diagnostic, File, FileId, FileType, ModuleId, PackageId, ProfileId, Uri};

/// One explicit file-content update applied through the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileUpdate {
    /// Replace file content with text.
    Text { content: String },
    /// Replace file content with raw bytes.
    Bytes { content: Vec<u8> },
    /// Remove the file from the revision.
    Removed,
}

/// One coarse impact kind for a service update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateImpactKind {
    /// One unknown or ordinary source change.
    Unknown,
    /// One `destack.json` change.
    Destack,
    /// One `tsconfig*.json` change.
    TsConfig,
}

/// One service-local impact summary for a file update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateImpact {
    /// The primary affected file id.
    pub file_id: FileId,
    /// The coarse impact kinds for this update.
    pub kinds: Vec<UpdateImpactKind>,
    /// The directly affected modules.
    pub modules: Vec<ModuleId>,
    /// The directly affected packages.
    pub packages: Vec<PackageId>,
    /// The directly affected profiles.
    pub profiles: Vec<ProfileId>,
    /// The dropped per-profile graph slices.
    pub graphs_dropped: Vec<ProfileId>,
}

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
    /// Publish uri for this update.
    pub publish_uri: Uri,
    /// Publish version for this update when it comes from one tracked open document.
    pub publish_version: Option<i32>,
    /// Updated file snapshot.
    pub file: FileSnapshot,
    /// Impact summary.
    pub impact: UpdateImpact,
    /// Diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostic snapshot for one document path.
#[derive(Debug, Clone)]
pub struct DocumentDiagnosticSnapshot {
    /// The current file snapshot used for range conversion.
    pub file: Arc<File>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Diagnostic snapshot for one workspace file.
#[derive(Debug, Clone)]
pub struct WorkspaceDiagnosticSnapshot {
    /// The current file snapshot used for range conversion.
    pub file: Arc<File>,
    /// Publish uri for this snapshot.
    pub publish_uri: Uri,
    /// Publish version for this snapshot when present.
    pub publish_version: Option<i32>,
    /// The diagnostics for this file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Result of applying local workspace service updates.
#[derive(Debug, Default)]
pub struct LanguageServiceResult {
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
