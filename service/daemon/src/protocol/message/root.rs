use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, DiagnosticBatch, RootHandleId};

/// Request to open a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenRootRequest {
    /// The root path.
    pub root: PathBuf,
    /// Root open options.
    pub options: RootOpenOptions,
}

/// Options for opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootOpenOptions {
    /// Whether to preload root state.
    pub load_index: bool,
}

impl Default for RootOpenOptions {
    fn default() -> Self {
        Self { load_index: true }
    }
}

/// Response to opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootOpenedResponse {
    /// Assigned root handle id.
    pub handle: RootHandleId,
    /// Diagnostics produced during initialization.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages produced during initialization.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to close a root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseRootRequest {
    /// Handle to close.
    pub handle: RootHandleId,
}

/// Response to closing a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootClosedResponse {
    /// Closed handle id.
    pub handle: RootHandleId,
}

/// Reason for a root reload request.
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

/// Request to reload a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReloadRootRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Response to root reloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RootReloadResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Updates produced during reload.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced during reload.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Request to apply a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Update payload.
    pub update: FileUpdate,
}

/// Response to a file update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUpdateResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Updates produced by the change.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the change.
    pub messages: Vec<DaemonMessageRecord>,
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
