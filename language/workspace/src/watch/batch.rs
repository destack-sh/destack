use std::time::Instant;

use destack_source::{FileWatchEvent, FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};

use crate::protocol::{WatchBatch, WatchEvent, WatchStatus};

/// Pending watch events before a batch is emitted.
#[derive(Debug, Clone)]
pub(super) struct PendingWatchBatch {
    /// The event list for this batch.
    events: Vec<WatchEvent>,
    /// The status updates for this batch.
    status: Vec<WatchStatus>,
    /// When the batch started.
    started_at: Instant,
    /// When the batch ended.
    pub(super) ended_at: Instant,
    /// Whether overflow events were observed.
    overflowed: bool,
}

impl PendingWatchBatch {
    /// Create an empty batch starting now.
    pub(super) fn new(started_at: Instant) -> Self {
        Self {
            events: Vec::new(),
            status: Vec::new(),
            started_at,
            ended_at: started_at,
            overflowed: false,
        }
    }

    /// Count the total items in the batch.
    pub(super) fn len(&self) -> usize {
        self.events.len() + self.status.len()
    }

    /// Record one source watcher item.
    pub(super) fn push(&mut self, item: SourceWatchItem) {
        match item {
            SourceWatchItem::Event(event) => {
                // mark overflow events
                if event.kind == FileWatchEventKind::Overflow {
                    self.overflowed = true;
                }

                self.events.push(event.into());
            }
            SourceWatchItem::Status(status) => {
                // mark overflow rescan requests
                if let FileWatchStatus::RescanRequested { reason, .. } = &status
                    && *reason == FileWatchRescanReason::Overflow
                {
                    self.overflowed = true;
                }

                self.status.push(status.into());
            }
        }
    }

    /// Finish the batch.
    pub(super) fn finish(self) -> WatchBatch {
        let duration = self.ended_at.saturating_duration_since(self.started_at);

        WatchBatch {
            events: self.events,
            status: self.status,
            overflowed: self.overflowed,
            started_at_ns: 0,
            ended_at_ns: duration.as_nanos() as u64,
        }
    }
}

/// Source watcher item.
pub(super) enum SourceWatchItem {
    /// A file event.
    Event(FileWatchEvent),
    /// A watcher status update.
    Status(FileWatchStatus),
}
