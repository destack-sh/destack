use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, RescanReason, WorkspaceHandleId};

/// Request to apply a watch batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchRequest {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Watch batch payload.
    pub batch: WatchBatch,
}

/// Response for watch batch processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Updates produced by the batch.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Whether a rescan is required.
    pub rescan: bool,
    /// Messages produced by the batch.
    pub messages: Vec<DaemonMessageRecord>,
}

/// Options for file watching within the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchOptions {
    /// Debounce interval in milliseconds.
    pub debounce_ms: u64,
    /// Optional poll interval in milliseconds.
    pub poll_interval_ms: Option<u64>,
    /// Whether to watch recursively.
    pub recursive: bool,
}

/// Watch batch payload used in the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatch {
    /// List of file events in the batch.
    pub events: Vec<WatchEvent>,
    /// Status updates emitted by the watcher.
    pub status: Vec<WatchStatus>,
    /// Whether overflow occurred.
    pub overflowed: bool,
    /// Batch start timestamp in unix nanoseconds.
    pub started_at_ns: u64,
    /// Batch end timestamp in unix nanoseconds.
    pub ended_at_ns: u64,
}

/// Watch event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchEvent {
    /// Event path.
    pub path: PathBuf,
    /// Optional previous path for renames.
    pub previous_path: Option<PathBuf>,
    /// Event kind.
    pub kind: WatchEventKind,
}

/// Watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventKind {
    /// Created event.
    Created,
    /// Modified event.
    Modified,
    /// Deleted event.
    Deleted,
    /// Renamed event.
    Renamed,
    /// Overflow event.
    Overflow,
}

/// Watch status update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchStatus {
    /// Watcher is ready.
    Ready { roots: Vec<PathBuf> },
    /// Watcher requests a rescan.
    RescanRequested {
        /// Watch roots for the rescan.
        roots: Vec<PathBuf>,
        /// Rescan reason.
        reason: RescanReason,
    },
    /// Watcher encountered an error.
    Error { message: String },
    /// Watcher stopped.
    Stopped,
}

/// Notification for watch updates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchUpdateNotification {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Update records.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Whether a rescan is required.
    pub rescan: bool,
}

/// Watch subscription request payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WatchRequest {
    /// Subscribe to watch notifications.
    Subscribe { handle: WorkspaceHandleId },
    /// Unsubscribe from watch notifications.
    Unsubscribe { handle: WorkspaceHandleId },
}

/// Watch subscription response payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchResponse {
    /// Workspace handle.
    pub handle: WorkspaceHandleId,
    /// Whether the subscription change succeeded.
    pub success: bool,
}
