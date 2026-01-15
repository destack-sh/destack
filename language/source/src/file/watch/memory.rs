use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use crossbeam_channel::bounded;
use parking_lot::RwLock;

use super::{
    FileWatchCommand, FileWatchCommandReceiver, FileWatchEvent, FileWatchFilter, FileWatchOptions,
    FileWatchRescanReason, FileWatchSender, FileWatchStatus, FileWatchStatusSender,
    FileWatchSubscription, FileWatchUpdate, FileWatcher,
};

/// In memory watcher that lets tests manually emit events.
#[derive(Default, Clone)]
pub struct MemoryFileWatcher {
    /// Active senders for registered watchers.
    entries: Arc<RwLock<HashMap<u64, MemoryWatchEntry>>>,
    /// Monotonic id generator for watch registrations.
    next_id: Arc<AtomicU64>,
}

impl std::fmt::Debug for MemoryFileWatcher {
    /// Format the memory watcher for debugging.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // snapshot active watcher count
        let entries = self.entries.read();

        // format watcher summary
        f.debug_struct("MemoryFileWatcher")
            .field("active", &entries.len())
            .finish()
    }
}

/// Entry for a memory watch subscription.
#[derive(Clone)]
struct MemoryWatchEntry {
    /// Sender for a single subscription.
    sender: FileWatchSender,
    /// Roots to filter events against.
    roots: Vec<PathBuf>,
    /// Optional filter for the subscription.
    filter: Option<FileWatchFilter>,
}

impl MemoryFileWatcher {
    /// Create a new MemoryFileWatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Emit a watch event to all active receivers.
    pub fn emit(&self, event: FileWatchEvent) {
        // broadcast to active entries
        let mut entries = self.entries.write();
        entries.retain(|_, entry| {
            if !should_emit_event(entry, &event) {
                return true;
            }
            entry.sender.send(event.clone()).is_ok()
        });
    }

    /// Emit a batch of watch events in order.
    pub fn emit_batch<I>(&self, events: I)
    where
        I: IntoIterator<Item = FileWatchEvent>,
    {
        // emit each event in sequence
        for event in events {
            self.emit(event);
        }
    }
}

impl FileWatcher for MemoryFileWatcher {
    fn watch(&self, roots: Vec<PathBuf>, options: FileWatchOptions) -> FileWatchSubscription {
        // setup channels
        let (sender, receiver) = bounded(options.channel_capacity);
        let (status_sender, status_receiver) = bounded(options.status_channel_capacity);
        let (command_sender, command_receiver) = bounded(options.command_channel_capacity);

        // register the watcher
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let entry = MemoryWatchEntry {
            sender,
            roots: roots.clone(),
            filter: options.filter.clone(),
        };
        self.entries.write().insert(id, entry);

        // emit startup status
        send_status(
            &status_sender,
            FileWatchStatus::Ready {
                roots: roots.clone(),
            },
        );
        send_status(
            &status_sender,
            FileWatchStatus::RescanRequested {
                roots: roots.clone(),
                reason: FileWatchRescanReason::Startup,
            },
        );

        // remove the watcher on stop
        let entries = Arc::clone(&self.entries);
        spawn_command_listener(entries, status_sender, command_receiver, id);

        FileWatchSubscription::new(receiver, status_receiver, command_sender)
    }
}

/// Spawn a listener that reacts to watch commands.
fn spawn_command_listener(
    entries: Arc<RwLock<HashMap<u64, MemoryWatchEntry>>>,
    status_sender: FileWatchStatusSender,
    command_receiver: FileWatchCommandReceiver,
    id: u64,
) {
    // listen for commands
    thread::spawn(move || {
        loop {
            // receive next command
            let command = match command_receiver.recv() {
                Ok(command) => command,
                Err(_) => break,
            };

            // handle command
            match command {
                FileWatchCommand::Stop => {
                    // remove entry on stop
                    entries.write().remove(&id);

                    // emit stopped status
                    send_status(&status_sender, FileWatchStatus::Stopped);
                    break;
                }
                FileWatchCommand::Rescan => {
                    // emit notice for manual rescan
                    let roots = current_roots(&entries, id);
                    send_status(
                        &status_sender,
                        FileWatchStatus::RescanRequested {
                            roots,
                            reason: FileWatchRescanReason::Manual,
                        },
                    );
                }
                FileWatchCommand::Update(update) => {
                    // apply updates and request rescan
                    let roots = apply_update(&entries, id, update);
                    send_status(
                        &status_sender,
                        FileWatchStatus::Ready {
                            roots: roots.clone(),
                        },
                    );
                    send_status(
                        &status_sender,
                        FileWatchStatus::RescanRequested {
                            roots,
                            reason: FileWatchRescanReason::Update,
                        },
                    );
                }
            }
        }
    });
}

/// Apply a watcher update and return updated roots.
fn apply_update(
    entries: &RwLock<HashMap<u64, MemoryWatchEntry>>,
    id: u64,
    update: FileWatchUpdate,
) -> Vec<PathBuf> {
    // update entry data
    let mut entries = entries.write();
    let Some(entry) = entries.get_mut(&id) else {
        return Vec::new();
    };

    // update roots when provided
    if let Some(roots) = update.roots {
        entry.roots = roots;
    }

    // update filter when provided
    if let Some(options) = update.options {
        entry.filter = options.filter;
    }

    entry.roots.clone()
}

/// Read the current roots for a watcher entry.
fn current_roots(entries: &RwLock<HashMap<u64, MemoryWatchEntry>>, id: u64) -> Vec<PathBuf> {
    // snapshot roots
    let entries = entries.read();
    entries
        .get(&id)
        .map(|entry| entry.roots.clone())
        .unwrap_or_default()
}

/// Send a status update without blocking.
fn send_status(status_sender: &FileWatchStatusSender, status: FileWatchStatus) {
    let _ = status_sender.try_send(status);
}

/// Check whether a watch event should be emitted for a filter and roots.
fn should_emit_event(entry: &MemoryWatchEntry, event: &FileWatchEvent) -> bool {
    // accept matches on the current path
    if matches_roots(&entry.roots, &event.path)
        && should_emit_filtered(entry.filter.as_ref(), &event.path)
    {
        return true;
    }

    // accept matches on the previous path
    event.previous_path.as_ref().is_some_and(|path| {
        matches_roots(&entry.roots, path) && should_emit_filtered(entry.filter.as_ref(), path)
    })
}

/// Check whether a path is accepted by roots.
fn matches_roots(roots: &[PathBuf], path: &Path) -> bool {
    // allow all paths when roots are empty
    if roots.is_empty() {
        return true;
    }

    roots.iter().any(|root| path.starts_with(root))
}

/// Check whether a path is accepted by a filter.
fn should_emit_filtered(filter: Option<&FileWatchFilter>, path: &Path) -> bool {
    // apply filter when present
    let Some(filter) = filter else {
        return true;
    };

    filter(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use super::super::{FileWatchEventKind, FileWatchRescanReason, FileWatchStatus};
    use super::*;

    /// Build a sample watch event.
    fn sample_event(path: &str) -> FileWatchEvent {
        FileWatchEvent {
            path: PathBuf::from(path),
            previous_path: None,
            kind: FileWatchEventKind::Modified,
        }
    }

    /// Drain a fixed number of status events.
    fn drain_status(subscription: &FileWatchSubscription, count: usize) {
        for _ in 0..count {
            let _ = subscription.status.recv_timeout(Duration::from_millis(50));
        }
    }

    /// Wait for a rescan requested status with the update reason.
    fn wait_for_update_rescan(subscription: &FileWatchSubscription) {
        // compute status deadline
        let deadline = Instant::now() + Duration::from_millis(200);
        while Instant::now() < deadline {
            // read next status
            let remaining = deadline.saturating_duration_since(Instant::now());
            let Ok(status) = subscription.status.recv_timeout(remaining) else {
                continue;
            };
            // check update rescan
            if matches!(
                status,
                FileWatchStatus::RescanRequested {
                    reason: FileWatchRescanReason::Update,
                    ..
                }
            ) {
                return;
            }
        }
        // fail when no update is observed
        panic!("expected update rescan status");
    }

    /// Emits events to active subscriptions.
    #[test]
    fn test_memory_watcher_emit() {
        let watcher = MemoryFileWatcher::new();
        let subscription = watcher.watch(Vec::new(), FileWatchOptions::default());

        // send event
        watcher.emit(sample_event("/workspace/file.ds"));

        // assertion block
        let received = subscription
            .receiver
            .recv_timeout(Duration::from_millis(50))
            .expect("expected watch event");
        assert_eq!(received.path, PathBuf::from("/workspace/file.ds"));
    }

    /// Filters events by predicate.
    #[test]
    fn test_memory_watcher_filter() {
        let watcher = MemoryFileWatcher::new();
        let mut options = FileWatchOptions::default();
        options.filter = Some(Arc::new(|path| path.ends_with(".ds")));
        let subscription = watcher.watch(Vec::new(), options);

        // send event outside the filter
        watcher.emit(sample_event("/workspace/file.ts"));

        // assertion block
        let result = subscription
            .receiver
            .recv_timeout(Duration::from_millis(50));
        assert!(result.is_err(), "expected no watch event");
    }

    /// Stops delivering events after stop is requested.
    #[test]
    fn test_memory_watcher_stop() {
        let watcher = MemoryFileWatcher::new();
        let subscription = watcher.watch(Vec::new(), FileWatchOptions::default());

        // stop and allow cleanup
        subscription.stop();
        thread::sleep(Duration::from_millis(10));

        // emit after stop
        watcher.emit(sample_event("/workspace/file.ds"));

        // assertion block
        let result = subscription
            .receiver
            .recv_timeout(Duration::from_millis(50));
        assert!(result.is_err(), "expected no watch event");
    }

    /// Updates roots and applies new root filters.
    #[test]
    fn test_memory_watcher_update_roots() {
        let watcher = MemoryFileWatcher::new();
        let subscription = watcher.watch(
            vec![PathBuf::from("/workspace/src")],
            FileWatchOptions::default(),
        );

        // drain startup status events
        drain_status(&subscription, 2);

        // emit outside root and assert no event
        watcher.emit(sample_event("/workspace/other/file.ds"));

        // assertion block
        let result = subscription
            .receiver
            .recv_timeout(Duration::from_millis(50));
        assert!(result.is_err(), "expected no watch event");

        // update roots to include new paths
        subscription.update(FileWatchUpdate::with_roots(vec![PathBuf::from(
            "/workspace",
        )]));
        wait_for_update_rescan(&subscription);

        // emit within updated roots
        watcher.emit(sample_event("/workspace/other/file.ds"));

        // assertion block
        let received = subscription
            .receiver
            .recv_timeout(Duration::from_millis(50))
            .expect("expected watch event");
        assert_eq!(received.path, PathBuf::from("/workspace/other/file.ds"));
    }
}
