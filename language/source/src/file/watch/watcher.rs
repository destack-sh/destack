use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

/// Default capacity for output event channels.
const DEFAULT_CHANNEL_CAPACITY: usize = 2048;
/// Default capacity for raw notify event channels.
const DEFAULT_RAW_CHANNEL_CAPACITY: usize = 8192;
/// Default capacity for status channels.
const DEFAULT_STATUS_CHANNEL_CAPACITY: usize = 128;
/// Default capacity for command channels.
const DEFAULT_COMMAND_CHANNEL_CAPACITY: usize = 32;

/// Filter predicate for watch events.
pub type FileWatchFilter = Arc<dyn Fn(&Path) -> bool + Send + Sync>;
/// Sender for file watch events.
pub type FileWatchSender = Sender<FileWatchEvent>;
/// Receiver for file watch events.
pub type FileWatchReceiver = Receiver<FileWatchEvent>;
/// Sender for file watch status events.
pub type FileWatchStatusSender = Sender<FileWatchStatus>;
/// Receiver for file watch status events.
pub type FileWatchStatusReceiver = Receiver<FileWatchStatus>;
/// Sender for file watch commands.
pub type FileWatchCommandSender = Sender<FileWatchCommand>;
/// Receiver for file watch commands.
pub type FileWatchCommandReceiver = Receiver<FileWatchCommand>;

/// Options for file watching.
#[derive(Clone)]
pub struct FileWatchOptions {
    /// The debounce interval used to coalesce events.
    pub debounce: Duration,
    /// Optional poll interval for fallback watchers.
    pub poll_interval: Option<Duration>,
    /// Whether to watch directories recursively.
    pub recursive: bool,
    /// Capacity of the output event channel.
    pub channel_capacity: usize,
    /// Capacity of the raw notify event channel.
    pub raw_channel_capacity: usize,
    /// Capacity of the status channel.
    pub status_channel_capacity: usize,
    /// Capacity of the command channel.
    pub command_channel_capacity: usize,
    /// Optional filter applied to watch events.
    pub filter: Option<FileWatchFilter>,
}

impl std::fmt::Debug for FileWatchOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileWatchOptions")
            .field("debounce", &self.debounce)
            .field("poll_interval", &self.poll_interval)
            .field("recursive", &self.recursive)
            .field("channel_capacity", &self.channel_capacity)
            .field("raw_channel_capacity", &self.raw_channel_capacity)
            .field("status_channel_capacity", &self.status_channel_capacity)
            .field("command_channel_capacity", &self.command_channel_capacity)
            .field("filter", &self.filter.is_some())
            .finish()
    }
}

impl Default for FileWatchOptions {
    fn default() -> Self {
        Self {
            debounce: Duration::from_millis(30),
            poll_interval: None,
            recursive: true,
            channel_capacity: DEFAULT_CHANNEL_CAPACITY,
            raw_channel_capacity: DEFAULT_RAW_CHANNEL_CAPACITY,
            status_channel_capacity: DEFAULT_STATUS_CHANNEL_CAPACITY,
            command_channel_capacity: DEFAULT_COMMAND_CHANNEL_CAPACITY,
            filter: None,
        }
    }
}

/// The kind of file watch event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileWatchEventKind {
    /// The path was created.
    Created,
    /// The path content changed.
    Modified,
    /// The path was removed.
    Deleted,
    /// The path was renamed or moved.
    Renamed,
    /// Events were dropped and the watcher lost fidelity.
    Overflow,
}

/// A file watch event emitted by a watcher.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileWatchEvent {
    /// The path associated with the event.
    pub path: PathBuf,
    /// The previous path for rename events.
    pub previous_path: Option<PathBuf>,
    /// The event kind.
    pub kind: FileWatchEventKind,
}

/// Reason a rescan was requested.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileWatchRescanReason {
    /// Requested when the watcher first starts.
    Startup,
    /// Requested because events were dropped.
    Overflow,
    /// Requested by the caller.
    Manual,
    /// Requested after watch roots or options changed.
    Update,
}

/// Status events emitted by a watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileWatchStatus {
    /// Watcher started and is ready.
    Ready { roots: Vec<PathBuf> },
    /// A rescan should be performed.
    RescanRequested {
        roots: Vec<PathBuf>,
        reason: FileWatchRescanReason,
    },
    /// A watcher error occurred.
    Error { message: String },
    /// The watcher has stopped.
    Stopped,
}

/// Watch command update payload.
#[derive(Debug, Clone)]
pub struct FileWatchUpdate {
    /// Updated watch roots.
    pub roots: Option<Vec<PathBuf>>,
    /// Updated watch options.
    pub options: Option<FileWatchOptions>,
}

impl FileWatchUpdate {
    /// Create an update with new roots.
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            roots: Some(roots),
            options: None,
        }
    }

    /// Create an update with new options.
    pub fn with_options(options: FileWatchOptions) -> Self {
        Self {
            roots: None,
            options: Some(options),
        }
    }
}

/// Command sent to an active watcher.
#[derive(Debug, Clone)]
pub enum FileWatchCommand {
    /// Stop the watcher.
    Stop,
    /// Request a rescan.
    Rescan,
    /// Update roots or options.
    Update(FileWatchUpdate),
}

/// Subscription returned by a watcher.
#[derive(Debug)]
pub struct FileWatchSubscription {
    /// Receiver for file watch events.
    pub receiver: FileWatchReceiver,
    /// Receiver for watch status updates.
    pub status: FileWatchStatusReceiver,
    /// Command sender for the watch loop.
    command: FileWatchCommandSender,
}

impl FileWatchSubscription {
    /// Create a new subscription from channels.
    pub(crate) fn new(
        receiver: FileWatchReceiver,
        status: FileWatchStatusReceiver,
        command: FileWatchCommandSender,
    ) -> Self {
        Self {
            receiver,
            status,
            command,
        }
    }

    /// Stop the watch loop.
    pub fn stop(&self) {
        let _ = self.command.send(FileWatchCommand::Stop);
    }

    /// Request a rescan for the watcher.
    pub fn rescan(&self) {
        let _ = self.command.send(FileWatchCommand::Rescan);
    }

    /// Update watch roots or options.
    pub fn update(&self, update: FileWatchUpdate) {
        let _ = self.command.send(FileWatchCommand::Update(update));
    }
}

impl Drop for FileWatchSubscription {
    fn drop(&mut self) {
        let _ = self.command.send(FileWatchCommand::Stop);
    }
}

/// Interface for file watchers used by workspace and daemon.
pub trait FileWatcher: Send + Sync {
    /// Start watching the given roots and return a subscription.
    fn watch(&self, roots: Vec<PathBuf>, options: FileWatchOptions) -> FileWatchSubscription;
}
