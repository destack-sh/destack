use std::path::PathBuf;

use destack_serde::Reflect;
use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};
use serde::{Deserialize, Serialize};

use crate::{ReloadReason, UpdateBatch};

/// Batch and workspace updates produced by one watch step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct WatchUpdate {
    /// Watch batch received from the watcher.
    pub batch: WatchBatch,
    /// Updates produced by applying the batch.
    pub updates: UpdateBatch,
}

/// One batch emitted by a workspace watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct WatchBatch {
    /// File events in the batch.
    pub events: Vec<WatchEvent>,
    /// Watcher status changes in the batch.
    pub status: Vec<WatchStatus>,
    /// Whether the watcher lost events.
    pub overflowed: bool,
    /// Time spent collecting this batch in nanoseconds.
    pub duration_nanoseconds: u64,
}

impl WatchBatch {
    /// Return whether the batch contains no events or status changes.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.status.is_empty()
    }
}

/// One file event emitted by a workspace watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct WatchEvent {
    /// Event path.
    pub path: PathBuf,
    /// Previous path for a rename.
    pub previous_path: Option<PathBuf>,
    /// Event kind.
    pub kind: WatchEventKind,
}

impl From<&WatchEvent> for FileWatchEvent {
    /// Convert one workspace watch event into a source event.
    fn from(event: &WatchEvent) -> Self {
        Self {
            path: event.path.clone(),
            previous_path: event.previous_path.clone(),
            kind: event.kind.into(),
        }
    }
}

impl From<FileWatchEvent> for WatchEvent {
    /// Convert one source event into a workspace watch event.
    fn from(event: FileWatchEvent) -> Self {
        Self {
            path: event.path,
            previous_path: event.previous_path,
            kind: event.kind.into(),
        }
    }
}

/// Kind of file change emitted by a workspace watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum WatchEventKind {
    /// A path was created.
    Created,
    /// A path changed.
    Modified,
    /// A path was deleted.
    Deleted,
    /// A path was renamed.
    Renamed,
    /// The watcher lost events.
    Overflow,
}

impl From<FileWatchEventKind> for WatchEventKind {
    /// Convert one source event kind into a workspace event kind.
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
    /// Convert one workspace event kind into a source event kind.
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

/// One status change emitted by a workspace watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum WatchStatus {
    /// The watcher is ready.
    Ready {
        /// Active watch roots.
        roots: Vec<PathBuf>,
    },
    /// The watcher requires a source reload.
    ReloadRequested {
        /// Roots requiring a reload.
        roots: Vec<PathBuf>,
        /// Reload reason.
        reason: ReloadReason,
    },
    /// The watcher encountered an error.
    Error {
        /// Human-readable error description.
        message: String,
    },
    /// The watcher stopped.
    Stopped,
}

impl From<FileWatchStatus> for WatchStatus {
    /// Convert one source watcher status into a workspace status.
    fn from(status: FileWatchStatus) -> Self {
        match status {
            FileWatchStatus::Ready { roots } => Self::Ready { roots },
            FileWatchStatus::RescanRequested { roots, reason } => Self::ReloadRequested {
                roots,
                reason: reason.into(),
            },
            FileWatchStatus::Error { message } => Self::Error { message },
            FileWatchStatus::Stopped => Self::Stopped,
        }
    }
}

impl From<FileWatchRescanReason> for ReloadReason {
    /// Convert one watcher rescan reason into a workspace reload reason.
    fn from(reason: FileWatchRescanReason) -> Self {
        match reason {
            FileWatchRescanReason::Startup | FileWatchRescanReason::Manual => Self::Manual,
            FileWatchRescanReason::Overflow => Self::Overflow,
            FileWatchRescanReason::Update => Self::Watch,
        }
    }
}

impl From<ReloadReason> for FileWatchRescanReason {
    /// Convert one workspace reload reason into a watcher rescan reason.
    fn from(reason: ReloadReason) -> Self {
        match reason {
            ReloadReason::Overflow => Self::Overflow,
            ReloadReason::Manual => Self::Manual,
            ReloadReason::Watch => Self::Update,
        }
    }
}
