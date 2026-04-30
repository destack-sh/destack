use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, ReloadReason, RootHandleId};

/// Request to apply a watch batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Watch batch payload.
    pub batch: WatchBatch,
}

/// Response for watch batch processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Updates produced by the batch.
    pub updates: Vec<DaemonUpdateRecord>,
    /// Messages produced by the batch.
    pub messages: Vec<DaemonMessageRecord>,
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
    /// Watcher requests a filesystem reload.
    ReloadRequested {
        /// Watch roots for the reload.
        roots: Vec<PathBuf>,
        /// Reload reason.
        reason: ReloadReason,
    },
    /// Watcher encountered an error.
    Error { message: String },
    /// Watcher stopped.
    Stopped,
}
