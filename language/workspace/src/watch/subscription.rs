use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::{after, select};
use destack_source::{FileWatchOptions, FileWatchSubscription, FileWatcher};

use super::batch::{PendingWatchBatch, SourceWatchItem};
use crate::protocol::WatchBatch;
use crate::{UpdateBatch, WatchPolicy};

/// Batch and workspace updates produced by one watch step.
#[derive(Debug, Clone)]
pub struct WatchUpdate {
    /// Watch batch received from the watcher.
    pub batch: WatchBatch,
    /// Updates produced by applying the batch.
    pub updates: UpdateBatch,
}

/// Active watch subscription with batching policy.
#[derive(Debug)]
pub struct Watch {
    /// The watcher subscription.
    subscription: FileWatchSubscription,
    /// The batching policy.
    policy: WatchPolicy,
}

impl Watch {
    /// Start watching roots with the provided policy.
    pub fn new(
        watcher: Arc<dyn FileWatcher>,
        roots: Vec<PathBuf>,
        options: FileWatchOptions,
        policy: WatchPolicy,
    ) -> Self {
        // start the watcher
        let subscription = watcher.watch(roots, options);

        Self {
            subscription,
            policy,
        }
    }

    /// Receive the next batch of events.
    pub fn next_batch(&self) -> Option<WatchBatch> {
        // wait for the first item
        let mut inbox = WatchInbox::new(&self.subscription);
        let first_item = inbox.recv()?;

        // initialize the batch with the first item
        let started_at = Instant::now();
        let mut batch = PendingWatchBatch::new(started_at);
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
            let Some(item) = inbox.recv_until(deadline) else {
                break;
            };
            batch.push(item);
        }

        // finalize timestamps
        batch.ended_at = Instant::now();

        // return the batch
        Some(batch.finish())
    }

    /// Stop the watcher.
    pub fn stop(&self) {
        self.subscription.stop();
    }
}

/// Receiver state for one watch batch.
struct WatchInbox<'a> {
    /// The watcher subscription.
    subscription: &'a FileWatchSubscription,
    /// Whether the event channel is closed.
    events_closed: bool,
    /// Whether the status channel is closed.
    status_closed: bool,
}

impl<'a> WatchInbox<'a> {
    /// Create receiver state for a subscription.
    fn new(subscription: &'a FileWatchSubscription) -> Self {
        Self {
            subscription,
            events_closed: false,
            status_closed: false,
        }
    }

    /// Wait for the next watch item.
    fn recv(&mut self) -> Option<SourceWatchItem> {
        loop {
            // stop when both channels are closed
            if self.events_closed && self.status_closed {
                return None;
            }

            // wait on the status channel when events are closed
            if self.events_closed {
                let status = self.subscription.status.recv().ok()?;

                return Some(SourceWatchItem::Status(status));
            }

            // wait on the events channel when status is closed
            if self.status_closed {
                let event = self.subscription.receiver.recv().ok()?;

                return Some(SourceWatchItem::Event(event));
            }

            // wait for either an event or a status update
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

            // return the first item we see
            if item.is_some() {
                return item;
            }
        }
    }

    /// Receive a watch item until the deadline expires.
    fn recv_until(&mut self, deadline: Instant) -> Option<SourceWatchItem> {
        loop {
            // stop when both channels are closed
            if self.events_closed && self.status_closed {
                return None;
            }

            // stop when the deadline has passed
            let timeout = deadline.saturating_duration_since(Instant::now());
            if timeout.is_zero() {
                return None;
            }

            // wait on the status channel when events are closed
            if self.events_closed {
                let status = self.subscription.status.recv_timeout(timeout).ok()?;

                return Some(SourceWatchItem::Status(status));
            }

            // wait on the events channel when status is closed
            if self.status_closed {
                let event = self.subscription.receiver.recv_timeout(timeout).ok()?;

                return Some(SourceWatchItem::Event(event));
            }

            // wait for an item or timeout
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
