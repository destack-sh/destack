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
    /// Whether to preload semantic workspace state.
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

/// Reason for a workspace reload request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReloadReason {
    /// Requested on startup.
    Startup,
    /// Requested after a watch overflow.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots changed.
    Update,
}

/// Request to reload a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReloadWorkspaceRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Response to workspace reloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceReloadResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced during reload.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced during reload.
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

/// Request to prepare query artifacts for a path within a workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareQueryRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Path to prepare.
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

/// Response to a prepare-query request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrepareQueryResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether query artifacts are ready after preparation.
    pub query_ready: bool,
    /// Optional readiness detail when query artifacts are not ready.
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
