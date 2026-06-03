use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{DaemonMessageRecord, DaemonUpdateRecord, ReloadReason, RootHandleId};

/// Request to start watching roots through a handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchStartRequest {
    /// Root handle.
    pub handle: RootHandleId,
    /// Roots watched through this handle.
    pub roots: Vec<PathBuf>,
    /// Watch start options.
    pub options: WatchStartOptions,
}

/// Options for starting a daemon watch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchStartOptions {
    /// Maximum time to coalesce events in milliseconds.
    pub coalesce_window_ms: u64,
    /// Maximum event and status count per batch.
    pub max_batch_size: usize,
}

/// Response for watch start requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchStartedResponse {
    /// Root handle.
    pub handle: RootHandleId,
}

/// Request to receive and apply the next watch batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchNextRequest {
    /// Root handle.
    pub handle: RootHandleId,
}

/// Request to stop watching roots through a handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchStopRequest {
    /// Root handle.
    pub handle: RootHandleId,
}

/// Response for watch stop requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchStoppedResponse {
    /// Root handle.
    pub handle: RootHandleId,
}

/// Response for watch batch processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchBatchResponse {
    /// Root handle.
    pub handle: RootHandleId,
    /// Watch batch received from the daemon watcher.
    pub batch: Option<WatchBatch>,
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
    /// Batch start timestamp in relative nanoseconds.
    pub started_at_ns: u64,
    /// Batch end timestamp in relative nanoseconds.
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
