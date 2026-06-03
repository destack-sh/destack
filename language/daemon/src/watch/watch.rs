use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{after, select};
use destack_source::{
    FileWatchEvent, FileWatchEventKind, FileWatchOptions, FileWatchRescanReason, FileWatchStatus,
    FileWatchSubscription, FileWatchUpdate, FileWatcher,
};

/// Default coalesce window in milliseconds.
const DEFAULT_COALESCE_WINDOW_MS: u64 = 50;
/// Default maximum batch size.
const DEFAULT_MAX_BATCH_SIZE: usize = 1024;

/// Policy for batching watch events.
#[derive(Debug, Clone)]
pub struct WatchPolicy {
    /// The maximum time to coalesce events into a batch.
    pub coalesce_window: Duration,
    /// The maximum number of events and status updates per batch.
    pub max_batch_size: usize,
}

impl Default for WatchPolicy {
    fn default() -> Self {
        Self {
            coalesce_window: Duration::from_millis(DEFAULT_COALESCE_WINDOW_MS),
            max_batch_size: DEFAULT_MAX_BATCH_SIZE,
        }
    }
}

/// Batch of watch events and status updates.
#[derive(Debug, Clone)]
pub struct WatchBatch {
    /// The event list for this batch.
    pub events: Vec<FileWatchEvent>,
    /// The status updates for this batch.
    pub status: Vec<FileWatchStatus>,
    /// When the batch started.
    pub started_at: Instant,
    /// When the batch ended.
    pub ended_at: Instant,
    /// Whether overflow events were observed.
    pub overflowed: bool,
}

impl WatchBatch {
    /// Create an empty batch starting now.
    pub fn new(started_at: Instant) -> Self {
        Self {
            events: Vec::new(),
            status: Vec::new(),
            started_at,
            ended_at: started_at,
            overflowed: false,
        }
    }

    /// Return true when the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty() && self.status.is_empty()
    }

    /// Count the total items in the batch.
    pub fn len(&self) -> usize {
        self.events.len() + self.status.len()
    }

    /// Get the elapsed batch duration.
    pub fn duration(&self) -> Duration {
        self.ended_at.saturating_duration_since(self.started_at)
    }

    /// Record one watch item into the batch.
    fn push(&mut self, item: WatchItem) {
        match item {
            WatchItem::Event(event) => {
                // mark overflow events
                if event.kind == FileWatchEventKind::Overflow {
                    self.overflowed = true;
                }

                self.events.push(event);
            }
            WatchItem::Status(status) => {
                // mark overflow rescan requests
                if let FileWatchStatus::RescanRequested { reason, .. } = &status
                    && *reason == FileWatchRescanReason::Overflow
                {
                    self.overflowed = true;
                }

                self.status.push(status);
            }
        }
    }
}

/// Coordinates watch event batching.
#[derive(Debug)]
pub struct WatchCoordinator {
    /// The watcher subscription.
    pub subscription: FileWatchSubscription,
    /// The batching policy.
    pub policy: WatchPolicy,
}

impl WatchCoordinator {
    /// Start watching roots with the provided policy.
    pub fn new(
        watcher: Arc<dyn FileWatcher>,
        roots: Vec<PathBuf>,
        options: FileWatchOptions,
        policy: WatchPolicy,
    ) -> Self {
        // start the watcher
        let subscription = watcher.watch(roots, options);

        // return the coordinator
        Self {
            subscription,
            policy,
        }
    }

    /// Receive the next batch of events.
    pub fn next_batch(&self) -> Option<WatchBatch> {
        // wait for the first item
        let first_item = self.wait_for_item()?;

        // initialize the batch with the first item
        let started_at = Instant::now();
        let mut batch = WatchBatch::new(started_at);
        batch.push(first_item);

        // continue collecting until the window expires or size is reached
        let deadline = started_at + self.policy.coalesce_window;
        // poll channels until an item arrives or both are closed
        loop {
            // stop when the batch is at capacity
            if batch.len() >= self.policy.max_batch_size {
                break;
            }

            // receive items until the deadline
            let Some(item) = self.recv_until(deadline) else {
                break;
            };
            batch.push(item);
        }

        // finalize timestamps
        batch.ended_at = Instant::now();

        // return the batch
        Some(batch)
    }

    /// Stop the watcher.
    pub fn stop(&self) {
        self.subscription.stop();
    }

    /// Request a rescan.
    pub fn rescan(&self) {
        self.subscription.rescan();
    }

    /// Update watch roots or options.
    pub fn update(&self, update: FileWatchUpdate) {
        self.subscription.update(update);
    }

    /// Wait for the next watch item.
    fn wait_for_item(&self) -> Option<WatchItem> {
        // track closed channels
        let mut events_closed = false;
        let mut status_closed = false;

        // poll channels until an item arrives or the deadline expires
        loop {
            // stop when both channels are closed
            if events_closed && status_closed {
                return None;
            }

            // wait on the status channel when events are closed
            if events_closed {
                let status = self.subscription.status.recv().ok()?;

                return Some(WatchItem::Status(status));
            }

            // wait on the events channel when status is closed
            if status_closed {
                let event = self.subscription.receiver.recv().ok()?;

                return Some(WatchItem::Event(event));
            }

            // wait for either an event or a status update
            let item = select! {
                recv(self.subscription.receiver) -> message => {
                    match message {
                        Ok(event) => Some(WatchItem::Event(event)),
                        Err(_) => {
                            events_closed = true;
                            None
                        }
                    }
                },
                recv(self.subscription.status) -> message => {
                    match message {
                        Ok(status) => Some(WatchItem::Status(status)),
                        Err(_) => {
                            status_closed = true;
                            None
                        }
                    }
                },
            };

            // return the first item we see
            if item.is_some() {
                return item;
            }
        }
    }

    /// Receive a watch item until the deadline expires.
    fn recv_until(&self, deadline: Instant) -> Option<WatchItem> {
        // track closed channels
        let mut events_closed = false;
        let mut status_closed = false;

        loop {
            // stop when both channels are closed
            if events_closed && status_closed {
                return None;
            }

            // stop when the deadline has passed
            let timeout = deadline.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                return None;
            }

            // wait on the status channel when events are closed
            if events_closed {
                let status = self.subscription.status.recv_timeout(timeout).ok()?;

                return Some(WatchItem::Status(status));
            }

            // wait on the events channel when status is closed
            if status_closed {
                let event = self.subscription.receiver.recv_timeout(timeout).ok()?;

                return Some(WatchItem::Event(event));
            }

            // wait for an item or timeout
            let timeout_receiver = after(timeout);
            let mut timed_out = false;
            let item = select! {
                recv(self.subscription.receiver) -> message => {
                    match message {
                        Ok(event) => Some(WatchItem::Event(event)),
                        Err(_) => {
                            events_closed = true;
                            None
                        }
                    }
                },
                recv(self.subscription.status) -> message => {
                    match message {
                        Ok(status) => Some(WatchItem::Status(status)),
                        Err(_) => {
                            status_closed = true;
                            None
                        }
                    }
                },
                recv(timeout_receiver) -> _ => {
                    timed_out = true;
                    None
                },
            };

            // return the item we see
            if item.is_some() {
                return item;
            }

            // stop after a timeout
            if timed_out {
                return None;
            }
        }
    }
}

/// Item received from the watcher channels.
enum WatchItem {
    /// A file event.
    Event(FileWatchEvent),
    /// A watcher status update.
    Status(FileWatchStatus),
}
