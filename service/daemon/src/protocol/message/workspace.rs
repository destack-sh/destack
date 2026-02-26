use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, DiagnosticBatch, WorkspaceHandleId};

/// Request to open a workspace root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenWorkspaceRequest {
    /// The workspace root path.
    pub root: PathBuf,
    /// Workspace open options.
    pub options: WorkspaceOpenOptions,
}

/// Options for opening a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOpenOptions {
    /// Whether to preload workspace index state.
    pub load_index: bool,
}

impl Default for WorkspaceOpenOptions {
    fn default() -> Self {
        Self { load_index: true }
    }
}

/// Response to opening a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceOpenedResponse {
    /// Assigned workspace handle id.
    pub handle: WorkspaceHandleId,
    /// Diagnostics produced during initialization.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages produced during initialization.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to close a workspace handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseWorkspaceRequest {
    /// Handle to close.
    pub handle: WorkspaceHandleId,
}

/// Response to closing a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceClosedResponse {
    /// Closed handle id.
    pub handle: WorkspaceHandleId,
}

/// Reason for a workspace rescan request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RescanReason {
    /// Requested on startup.
    Startup,
    /// Requested after a watch overflow.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots changed.
    Update,
}

/// Request to rescan a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RescanWorkspaceRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Reason for the rescan.
    pub reason: RescanReason,
}

/// Response to workspace rescans.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceRescanResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced during rescan.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced during rescan.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to apply a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Update payload.
    pub update: FileUpdate,
}

/// Request to analyze a path within a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Path to analyze.
    pub path: PathBuf,
}

/// Response to a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced by the change.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the change.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Response to an analyze request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether semantic query state is ready after analysis.
    pub semantic_query_ready: bool,
    /// Optional readiness detail when semantic query state is not ready.
    pub detail: Option<String>,
}

/// File update payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdate {
    /// Path being updated.
    pub path: PathBuf,
    /// The update payload.
    pub update: FileUpdateKind,
    /// Whether to write to disk.
    pub write_to_disk: bool,
}

/// File update kinds for content changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileUpdateKind {
    /// Replace with new text content.
    Text { content: String },
    /// Replace with new binary content.
    Bytes { content: Vec<u8> },
    /// Touch the file version without modifying content.
    Touch,
    /// Mark the file as missing.
    Removed,
}
