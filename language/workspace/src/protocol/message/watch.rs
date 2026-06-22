use std::path::PathBuf;
use std::time::Duration;

use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};

use super::RootId;
use crate::{ReloadReason, UpdateBatch};

/// Default watch coalesce window in milliseconds.
const DEFAULT_WATCH_COALESCE_WINDOW_MS: u64 = 50;

/// Default maximum watch batch size.
const DEFAULT_WATCH_BATCH_SIZE: usize = 1024;

/// Policy for batching workspace watch events.
#[derive(Debug, Clone)]
pub struct WatchPolicy {
    /// The maximum time to coalesce events into a batch.
    pub coalesce_window: Duration,
    /// The maximum number of events and status updates per batch.
    pub max_batch_size: usize,
}

impl WatchPolicy {
    /// Convert this policy into protocol watch start options.
    pub fn start_options(&self) -> WatchStartOptions {
        WatchStartOptions {
            coalesce_window_ms: self.coalesce_window.as_millis() as u64,
            max_batch_size: self.max_batch_size,
        }
    }
}

impl Default for WatchPolicy {
    /// Return the default watch policy.
    fn default() -> Self {
        Self {
            coalesce_window: Duration::from_millis(DEFAULT_WATCH_COALESCE_WINDOW_MS),
            max_batch_size: DEFAULT_WATCH_BATCH_SIZE,
        }
    }
}

/// Request to start watching roots through a handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct WatchStartRequest {
    /// Root handle.
    pub handle: RootId,
    /// Roots watched through this handle.
    pub roots: Vec<PathBuf>,
    /// Watch start options.
    pub options: WatchStartOptions,
}

/// Options for starting a workspace watch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct WatchStartOptions {
    /// Maximum time to coalesce events in milliseconds.
    pub coalesce_window_ms: u64,
    /// Maximum event and status count per batch.
    pub max_batch_size: usize,
}

impl WatchStartOptions {
    /// Convert this protocol shape into a workspace watch policy.
    pub fn policy(&self) -> WatchPolicy {
        WatchPolicy {
            coalesce_window: Duration::from_millis(self.coalesce_window_ms),
            max_batch_size: self.max_batch_size,
        }
    }
}

/// Response for watch start requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct WatchStartedResponse {
    /// Root handle.
    pub handle: RootId,
}

/// Request to receive and apply the next watch batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct WatchNextRequest {
    /// Root handle.
    pub handle: RootId,
}

/// Request to stop watching roots through a handle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct WatchStopRequest {
    /// Root handle.
    pub handle: RootId,
}

/// Response for watch stop requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
pub struct WatchStoppedResponse {
    /// Root handle.
    pub handle: RootId,
}

/// Response for watch batch processing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct WatchBatchResponse {
    /// Root handle.
    pub handle: RootId,
    /// Watch batch received from the workspace watcher.
    pub batch: Option<WatchBatch>,
    /// Updates produced by the batch.
    pub updates: UpdateBatch,
}

impl WatchBatchResponse {
    /// Build an empty watch batch response.
    pub fn empty(handle: RootId) -> Self {
        Self {
            handle,
            batch: None,
            updates: UpdateBatch::default(),
        }
    }
}

/// Watch batch payload used in the protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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

impl WatchBatch {
    /// Return true when the batch has no events or status updates.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.status.is_empty()
    }
}

/// Watch event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
pub struct WatchEvent {
    /// Event path.
    pub path: PathBuf,
    /// Optional previous path for renames.
    pub previous_path: Option<PathBuf>,
    /// Event kind.
    pub kind: WatchEventKind,
}

impl From<&WatchEvent> for FileWatchEvent {
    /// Convert a protocol watch event into source shape.
    fn from(event: &WatchEvent) -> Self {
        FileWatchEvent {
            path: event.path.clone(),
            previous_path: event.previous_path.clone(),
            kind: event.kind.into(),
        }
    }
}

impl From<FileWatchEvent> for WatchEvent {
    /// Convert a source watch event into protocol shape.
    fn from(event: FileWatchEvent) -> Self {
        Self {
            path: event.path,
            previous_path: event.previous_path,
            kind: WatchEventKind::from(event.kind),
        }
    }
}

/// Watch event kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Schema)]
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

impl From<FileWatchEventKind> for WatchEventKind {
    /// Convert a source watch event kind into protocol shape.
    fn from(kind: FileWatchEventKind) -> Self {
        match kind {
            FileWatchEventKind::Created => Self::Created,
            FileWatchEventKind::Modified => Self::Modified,
            FileWatchEventKind::Deleted => Self::Deleted,
            FileWatchEventKind::Renamed => Self::Renamed,
            FileWatchEventKind::Overflow => Self::Overflow,
        }
    }
}

impl From<WatchEventKind> for FileWatchEventKind {
    /// Convert a protocol watch event kind into source shape.
    fn from(kind: WatchEventKind) -> Self {
        match kind {
            WatchEventKind::Created => Self::Created,
            WatchEventKind::Modified => Self::Modified,
            WatchEventKind::Deleted => Self::Deleted,
            WatchEventKind::Renamed => Self::Renamed,
            WatchEventKind::Overflow => Self::Overflow,
        }
    }
}

/// Watch status update.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Schema)]
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

impl From<FileWatchStatus> for WatchStatus {
    /// Convert a source watch status into protocol shape.
    fn from(status: FileWatchStatus) -> Self {
        match status {
            FileWatchStatus::Ready { roots } => Self::Ready { roots },
            FileWatchStatus::RescanRequested { roots, reason } => Self::ReloadRequested {
                roots,
                reason: ReloadReason::from(reason),
            },
            FileWatchStatus::Error { message } => Self::Error { message },
            FileWatchStatus::Stopped => Self::Stopped,
        }
    }
}

impl From<FileWatchRescanReason> for ReloadReason {
    /// Convert a source watch rescan reason into workspace shape.
    fn from(reason: FileWatchRescanReason) -> Self {
        match reason {
            FileWatchRescanReason::Startup => Self::Manual,
            FileWatchRescanReason::Overflow => Self::Overflow,
            FileWatchRescanReason::Manual => Self::Manual,
            FileWatchRescanReason::Update => Self::Watch,
        }
    }
}

impl From<ReloadReason> for FileWatchRescanReason {
    /// Convert a workspace reload reason into source watch shape.
    fn from(reason: ReloadReason) -> Self {
        match reason {
            ReloadReason::Overflow => Self::Overflow,
            ReloadReason::Manual => Self::Manual,
            ReloadReason::Watch => Self::Update,
        }
    }
}
