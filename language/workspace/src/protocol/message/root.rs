use destack_serde::Schema;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::DiagnosticBatch;
use crate::{Commit, FileOperation, Message, ReloadReason, SourceUpdate, UpdateBatch};

/// Unique identifier for an opened root.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct RootId(pub u64);

impl RootId {
    /// Wrap a raw root handle id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Request to open a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct OpenRootRequest {
    /// The root path.
    pub root: PathBuf,
    /// Root open options.
    pub options: RootOpenOptions,
}

/// Options for opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct RootOpenOptions {
    /// Whether to preload root state.
    pub load_index: bool,
}

impl Default for RootOpenOptions {
    /// Return default root open options.
    fn default() -> Self {
        Self { load_index: true }
    }
}

/// Response to opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct RootOpenedResponse {
    /// Assigned root handle id.
    pub handle: RootId,
    /// Canonical root path opened by the server.
    pub root: PathBuf,
    /// Diagnostics produced during initialization.
    pub diagnostics: Vec<DiagnosticBatch>,
    /// Messages produced during initialization.
    pub messages: Vec<Message>,
}

/// Request to close a root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct CloseRootRequest {
    /// Handle to close.
    pub handle: RootId,
}

/// Response to closing a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct RootClosedResponse {
    /// Closed handle id.
    pub handle: RootId,
}

/// Request to reload a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct ReloadRootRequest {
    /// Root handle.
    pub handle: RootId,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Response to root reloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct RootReloadResponse {
    /// Root handle.
    pub handle: RootId,
    /// Updates produced during reload.
    pub updates: UpdateBatch,
}

/// Request to apply a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct FileOperationRequest {
    /// Root handle.
    pub handle: RootId,
    /// Operation payload.
    pub operation: FileOperation,
}

/// Response to a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct FileOperationResponse {
    /// Root handle.
    pub handle: RootId,
    /// Updates produced by the change.
    pub updates: UpdateBatch,
}

/// Request to apply a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct SourceUpdateRequest {
    /// Root handle.
    pub handle: RootId,
    /// Source update payload.
    pub update: SourceUpdate,
}

/// Response to a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct SourceUpdateResponse {
    /// Root handle.
    pub handle: RootId,
    /// Commit produced by the change.
    pub commit: Commit,
}
