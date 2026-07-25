use destack_serde::Reflect;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Commit, FileOperation, ReloadReason, SourceUpdate, UpdateBatch};

/// Unique identifier for an opened root.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct RootId(pub u64);

impl RootId {
    /// Wrap a raw root handle id.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Request to open a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct OpenRootRequest {
    /// The root path.
    pub root: PathBuf,
}

/// Response to opening a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RootOpenedResponse {
    /// Assigned root handle id.
    pub handle: RootId,
    /// Canonical root path opened by the server.
    pub root: PathBuf,
}

/// Request to close a root handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CloseRootRequest {
    /// Handle to close.
    pub handle: RootId,
}

/// Response to closing a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RootClosedResponse {
    /// Closed handle id.
    pub handle: RootId,
}

/// Request to reload a root.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ReloadRootRequest {
    /// Root handle.
    pub handle: RootId,
    /// Reason for the reload.
    pub reason: ReloadReason,
}

/// Response to root reloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RootReloadResponse {
    /// Root handle.
    pub handle: RootId,
    /// Updates produced during reload.
    pub updates: UpdateBatch,
}

/// Request to apply a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileOperationRequest {
    /// Root handle.
    pub handle: RootId,
    /// Operation payload.
    pub operation: FileOperation,
}

/// Response to a file operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct FileOperationResponse {
    /// Root handle.
    pub handle: RootId,
    /// Updates produced by the change.
    pub updates: UpdateBatch,
}

/// Request to apply a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SourceUpdateRequest {
    /// Root handle.
    pub handle: RootId,
    /// Source update payload.
    pub update: SourceUpdate,
}

/// Response to a source update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SourceUpdateResponse {
    /// Root handle.
    pub handle: RootId,
    /// Commit produced by the change.
    pub commit: Commit,
}
