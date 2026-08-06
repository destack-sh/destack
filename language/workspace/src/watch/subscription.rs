use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::{after, select};
use destack_source::{FileWatchOptions, FileWatchSubscription, FileWatcher};
use futures::channel::mpsc::{Receiver, Sender, channel};
use futures::executor::block_on;
use futures::lock::Mutex;
use futures::{SinkExt, StreamExt};

use super::batch::{PendingWatchBatch, SourceWatchItem};
use crate::{WatchBatch, WatchPolicy};

/// Maximum pending batch count for one workspace watch.
const WATCH_BATCH_CAPACITY: usize = 1;

/// Active batched workspace watch.
#[derive(Debug)]
pub(crate) struct WatchSubscription {
    /// Watched roots.
    roots: Vec<PathBuf>,
    /// Source watcher subscription.
    source: Arc<FileWatchSubscription>,
    /// Batches received from the blocking source watcher.
    batches: Mutex<Receiver<WatchBatch>>,
}

impl WatchSubscription {
    /// Start watching roots with the provided policy.
    pub(crate) fn new(
        watcher: Arc<dyn FileWatcher>,
        roots: Vec<PathBuf>,
        options: FileWatchOptions,
        policy: WatchPolicy,
    ) -> Self {
        // start the source watcher and bounded batch queue
        let source = Arc::new(watcher.watch(roots.clone(), options));
        let (batches, receiver) = channel(WATCH_BATCH_CAPACITY);
        let batch_source = source.clone();

        // isolate blocking source receives from async workspace consumers
        std::thread::spawn(move || {
            Self::forward_batches(batch_source, policy, batches);
        });

        Self {
            roots,
            source,
            batches: Mutex::new(receiver),
        }
    }

    /// Return whether this subscription watches one root.
    pub(crate) fn watches(&self, root: &Path) -> bool {
        self.roots.iter().any(|watch_root| watch_root == root)
    }

    /// Receive the next batch of events.
    pub(crate) async fn next_batch(&self) -> Option<WatchBatch> {
        self.batches.lock().await.next().await
    }

    /// Stop the source watcher.
    pub(crate) fn stop(&self) {
        self.source.stop();
    }

    /// Forward blocking source events into bounded asynchronous batches.
    fn forward_batches(
        source: Arc<FileWatchSubscription>,
        policy: WatchPolicy,
        mut batches: Sender<WatchBatch>,
    ) {
        let mut batcher = WatchBatcher::new(source, policy);

        // preserve backpressure through the bounded async queue
        while let Some(batch) = batcher.next_batch() {
            if block_on(batches.send(batch)).is_err() {
                return;
            }
        }
    }
}

impl Drop for WatchSubscription {
    /// Stop the source watcher when its workspace subscription closes.
    fn drop(&mut self) {
        self.source.stop();
    }
}

/// Blocking source watcher batcher.
struct WatchBatcher {
    /// Receiver state for the source subscription.
    inbox: WatchInbox,
    /// Workspace batching policy.
    policy: WatchPolicy,
}

impl WatchBatcher {
    /// Create one source watcher batcher.
    fn new(subscription: Arc<FileWatchSubscription>, policy: WatchPolicy) -> Self {
        Self {
            inbox: WatchInbox::new(subscription),
            policy,
        }
    }

    /// Receive the next batch of events.
    fn next_batch(&mut self) -> Option<WatchBatch> {
        // wait for the event that opens this batch
        let first_item = self.inbox.receive()?;
        let started_at = Instant::now();
        let mut batch = PendingWatchBatch::new(started_at);
        batch.push(first_item);

        // collect until the time window expires or the batch reaches capacity
        let deadline = started_at + self.policy.coalesce_window;
        while batch.len() < self.policy.max_batch_size {
            let Some(item) = self.inbox.receive_until(deadline) else {
                break;
            };
            batch.push(item);
        }

        // record the complete collection duration
        Some(batch.finish(Instant::now()))
    }
}

/// Receiver state for one source watch subscription.
struct WatchInbox {
    /// Source watcher subscription.
    subscription: Arc<FileWatchSubscription>,
    /// Whether the event receiver is closed.
    events_closed: bool,
    /// Whether the status receiver is closed.
    status_closed: bool,
}

impl WatchInbox {
    /// Create receiver state for a subscription.
    fn new(subscription: Arc<FileWatchSubscription>) -> Self {
        Self {
            subscription,
            events_closed: false,
            status_closed: false,
        }
    }

    /// Wait for the next watch item.
    fn receive(&mut self) -> Option<SourceWatchItem> {
        loop {
            // stop when both receivers are closed
            if self.events_closed && self.status_closed {
                return None;
            }

            // wait on the remaining receiver
            if self.events_closed {
                let status = self.subscription.status.recv().ok()?;

                return Some(SourceWatchItem::Status(status));
            } else if self.status_closed {
                let event = self.subscription.receiver.recv().ok()?;

                return Some(SourceWatchItem::Event(event));
            }

            // wait for either an event or a status change
            let item = select! {
                recv(self.subscription.receiver) -> message => {
                    match message {
                        Ok(event) => Some(SourceWatchItem::Event(event)),
                        Err(_) => {
                            self.events_closed = true;
                            None
                        }
                    }
                },
                recv(self.subscription.status) -> message => {
                    match message {
                        Ok(status) => Some(SourceWatchItem::Status(status)),
                        Err(_) => {
                            self.status_closed = true;
                            None
                        }
                    }
                },
            };

            if item.is_some() {
                return item;
            }
        }
    }

    /// Receive a watch item until the deadline expires.
    fn receive_until(&mut self, deadline: Instant) -> Option<SourceWatchItem> {
        loop {
            // stop when both receivers are closed
            if self.events_closed && self.status_closed {
                return None;
            }

            // stop when the deadline has passed
            let timeout = deadline.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                return None;
            }

            // wait on the remaining receiver
            if self.events_closed {
                let status = self.subscription.status.recv_timeout(timeout).ok()?;

                return Some(SourceWatchItem::Status(status));
            } else if self.status_closed {
                let event = self.subscription.receiver.recv_timeout(timeout).ok()?;

                return Some(SourceWatchItem::Event(event));
            }

            // wait for either source receiver or the batch deadline
            let timeout_receiver = after(timeout);
            let mut timed_out = false;
            let item = select! {
                recv(self.subscription.receiver) -> message => {
                    match message {
                        Ok(event) => Some(SourceWatchItem::Event(event)),
                        Err(_) => {
                            self.events_closed = true;
                            None
                        }
                    }
                },
                recv(self.subscription.status) -> message => {
                    match message {
                        Ok(status) => Some(SourceWatchItem::Status(status)),
                        Err(_) => {
                            self.status_closed = true;
                            None
                        }
                    }
                },
                recv(timeout_receiver) -> _ => {
                    timed_out = true;
                    None
                },
            };

            if item.is_some() {
                return item;
            }

            if timed_out {
                return None;
            }
        }
    }
}
