use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crossbeam_channel::{Receiver, bounded};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;

use super::{FileWatchError, FileWatchEvent};

/// Active operating-system file watch.
pub struct FileWatch {
    /// Wake signal for pending physical changes.
    pub changes: Receiver<()>,
    /// Runtime host watcher failures.
    pub errors: Receiver<FileWatchError>,
    /// Live host watcher registration.
    watcher: RecommendedWatcher,
    /// Registered recursive roots.
    roots: HashSet<PathBuf>,
    /// Deduplicated physical changes awaiting reconciliation.
    pending: Arc<Mutex<ChangeSet>>,
}

impl std::fmt::Debug for FileWatch {
    /// Format visible file watch state.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FileWatch")
            .field("roots", &self.roots)
            .field("pending", &self.pending)
            .finish_non_exhaustive()
    }
}

impl FileWatch {
    /// Create one empty physical file watch.
    pub fn new() -> Result<Self, FileWatchError> {
        let (change_sender, changes) = bounded(1);
        let (error_sender, errors) = bounded(1);
        let pending = Arc::new(Mutex::new(ChangeSet::default()));

        // merge host observations and wake the owner once
        let callback_pending = pending.clone();
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<Event>| match result {
                Ok(event) if matches!(event.kind, EventKind::Access(_)) => {}
                Ok(event) => {
                    let is_rescan = event.need_rescan();
                    if event.paths.is_empty() && !is_rescan {
                        return;
                    }

                    let mut pending = callback_pending.lock();
                    pending.paths.extend(event.paths);
                    pending.is_rescan |= is_rescan;
                    let is_notified = pending.is_notified;
                    pending.is_notified = true;
                    drop(pending);

                    if !is_notified {
                        let _ = change_sender.try_send(());
                    }
                }
                Err(error) => {
                    let _ = error_sender.try_send(error.into());
                }
            },
            notify::Config::default(),
        )?;

        Ok(Self {
            changes,
            errors,
            watcher,
            roots: HashSet::new(),
            pending,
        })
    }

    /// Watch one root recursively and return whether it was newly registered.
    pub fn watch(&mut self, root: &Path) -> Result<bool, FileWatchError> {
        if self.roots.contains(root) {
            return Ok(false);
        }

        self.watcher.watch(root, RecursiveMode::Recursive)?;
        self.roots.insert(root.to_path_buf());

        Ok(true)
    }

    /// Stop watching one root and return whether it was registered.
    pub fn unwatch(&mut self, root: &Path) -> Result<bool, FileWatchError> {
        if !self.roots.contains(root) {
            return Ok(false);
        }

        self.watcher.unwatch(root)?;
        self.roots.remove(root);

        Ok(true)
    }

    /// Take every physical change accumulated before this call.
    pub fn take(&self) -> FileWatchEvent {
        let mut pending = self.pending.lock();
        let mut paths = pending.paths.drain().collect::<Vec<_>>();
        paths.sort_unstable();
        let is_rescan = std::mem::take(&mut pending.is_rescan);
        pending.is_notified = false;

        FileWatchEvent { paths, is_rescan }
    }
}

/// Deduplicated physical changes awaiting reconciliation.
#[derive(Debug, Default)]
struct ChangeSet {
    /// Distinct changed paths.
    paths: HashSet<PathBuf>,
    /// Whether the host requires an authoritative rescan.
    is_rescan: bool,
    /// Whether the owner has an outstanding wake signal.
    is_notified: bool,
}
