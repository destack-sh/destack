use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{
    FileSystem, FileWatchEvent, FileWatchEventKind, FileWatchOptions, MemoryFileSystem,
    MemoryFileWatcher,
};
use destack_workspace::{MemoryCacheStore, Program, Session};

use crate::{Daemon, DaemonUpdate, WatchBatch, WatchCoordinator, WatchPolicy};

/// Test harness for daemon flows.
#[derive(Debug)]
pub struct TestDaemon {
    /// The in memory file system.
    pub fs: Arc<MemoryFileSystem>,
    /// The in memory file watcher.
    pub watcher: Arc<MemoryFileWatcher>,
    /// The shared session.
    pub session: Arc<Session>,
    /// The daemon under test.
    pub daemon: Daemon,
    /// The primary workspace root.
    pub root: PathBuf,
    /// The workspace roots for the daemon.
    roots: Vec<PathBuf>,
}

/// Harness for watch coordination in tests.
#[derive(Debug)]
pub struct TestWatchHarness {
    /// The daemon test state.
    pub test: TestDaemon,
    /// The watch coordinator under test.
    coordinator: WatchCoordinator,
}

/// Wrapper for watch batches in tests.
#[derive(Debug)]
pub struct TestWatchBatch {
    /// The captured watch batch.
    batch: WatchBatch,
}

impl TestDaemon {
    /// Create a test daemon with a default root.
    pub fn new() -> Self {
        Self::new_with_roots(vec![PathBuf::from("/workspace")])
    }

    /// Create a test daemon with explicit workspace roots.
    pub fn new_with_roots(mut roots: Vec<PathBuf>) -> Self {
        // ensure we have a primary root
        let root = roots
            .first()
            .cloned()
            .unwrap_or_else(|| PathBuf::from("/workspace"));
        if roots.is_empty() {
            roots.push(root.clone());
        }

        // build shared state
        let fs = Arc::new(MemoryFileSystem::new());
        let watcher = Arc::new(MemoryFileWatcher::new());
        let session = Arc::new(
            Session::new(root.clone())
                .with_fs(fs.clone())
                .with_cache_store(Arc::new(MemoryCacheStore::new())),
        );
        for root_path in &roots {
            session.add_root(root_path.clone());
        }
        let daemon = Daemon::new(session.clone());

        Self {
            fs,
            watcher,
            session,
            daemon,
            root,
            roots,
        }
    }

    /// Add an additional workspace root.
    pub fn add_root(&self, root: PathBuf) -> Arc<Program> {
        self.session.add_root(root)
    }

    /// Write a text file into the test file system.
    pub fn write_text(&self, path: impl AsRef<Path>, contents: &str) -> PathBuf {
        let path = self.resolve_path(path);
        if let Some(parent) = path.parent() {
            let _ = self.fs.create_dir_all(parent);
        }
        self.fs
            .write(&path, contents.as_bytes())
            .unwrap_or_else(|error| panic!("write failed for {}: {error}", path.display()));
        path
    }

    /// Update a file and return the daemon update.
    pub fn update_file(&self, path: impl AsRef<Path>, content: &str) -> DaemonUpdate {
        let path = self.resolve_path(path);
        self.daemon
            .update_file(&path, content.to_string())
            .unwrap_or_else(|error| panic!("update failed for {}: {error}", path.display()))
    }

    /// Build a watch coordinator for the test roots.
    pub fn watch_coordinator(&self, policy: WatchPolicy) -> WatchCoordinator {
        WatchCoordinator::new(
            self.watcher.clone(),
            self.roots.clone(),
            FileWatchOptions::default(),
            policy,
        )
    }

    /// Build a watch event for tests.
    pub fn watch_event(&self, path: impl AsRef<Path>, kind: FileWatchEventKind) -> FileWatchEvent {
        FileWatchEvent {
            path: self.resolve_path(path),
            previous_path: None,
            kind,
        }
    }

    /// Build a rename watch event for tests.
    pub fn watch_rename_event(
        &self,
        from: impl AsRef<Path>,
        to: impl AsRef<Path>,
    ) -> FileWatchEvent {
        FileWatchEvent {
            path: self.resolve_path(to),
            previous_path: Some(self.resolve_path(from)),
            kind: FileWatchEventKind::Renamed,
        }
    }

    /// Apply a watch batch to the daemon and return updates.
    pub fn apply_watch_batch(&self, batch: &WatchBatch) -> Vec<DaemonUpdate> {
        let mut updates = Vec::new();
        for event in &batch.events {
            let content = match self.session.fs.read_to_string(&event.path) {
                Ok(content) => content,
                Err(_) => continue,
            };
            let update = match self.daemon.update_file(&event.path, content) {
                Ok(update) => update,
                Err(_) => continue,
            };
            updates.push(update);
        }
        updates
    }

    /// Resolve a path relative to the primary root when needed.
    fn resolve_path(&self, path: impl AsRef<Path>) -> PathBuf {
        resolve_test_path(&self.root, path)
    }
}

impl TestWatchHarness {
    /// Create a watch harness with the provided policy.
    pub fn new(policy: WatchPolicy) -> Self {
        let test = TestDaemon::new();
        let coordinator = test.watch_coordinator(policy);
        Self { test, coordinator }
    }

    /// Consume the initial watch batch.
    pub fn skip_startup(&self) {
        let _ = self
            .coordinator
            .next_batch()
            .expect("expected startup batch");
    }

    /// Emit a watch event.
    pub fn emit(&self, path: impl AsRef<Path>, kind: FileWatchEventKind) {
        let event = self.test.watch_event(path, kind);
        self.test.watcher.emit(event);
    }

    /// Emit a rename watch event.
    pub fn emit_rename(&self, from: impl AsRef<Path>, to: impl AsRef<Path>) {
        let event = self.test.watch_rename_event(from, to);
        self.test.watcher.emit(event);
    }

    /// Return the next watch batch.
    pub fn next_batch(&self) -> TestWatchBatch {
        let batch = self.coordinator.next_batch().expect("expected watch batch");
        TestWatchBatch::new(batch)
    }

    /// Apply a watch batch and return daemon updates.
    pub fn apply_batch(&self, batch: &TestWatchBatch) -> Vec<DaemonUpdate> {
        self.test.apply_watch_batch(batch.batch())
    }

    /// Stop the watch coordinator.
    pub fn stop(&self) {
        self.coordinator.stop();
    }
}

impl TestWatchBatch {
    /// Wrap a watch batch for test assertions.
    pub fn new(batch: WatchBatch) -> Self {
        Self { batch }
    }

    /// Return the number of events in the batch.
    pub fn event_count(&self) -> usize {
        self.batch.events.len()
    }

    /// Return true when the batch has any status updates.
    pub fn has_status(&self) -> bool {
        !self.batch.status.is_empty()
    }

    /// Return true when the batch overflowed.
    pub fn overflowed(&self) -> bool {
        self.batch.overflowed
    }

    /// Return a reference to the underlying batch.
    pub fn batch(&self) -> &WatchBatch {
        &self.batch
    }

    /// Assert the batch contains a path suffix.
    pub fn assert_event_suffix(&self, suffix: &str) {
        assert!(
            self.batch
                .events
                .iter()
                .any(|event| event.path.ends_with(suffix)),
            "expected {suffix} to be present"
        );
    }
}

/// Resolve a path relative to the provided root when needed.
fn resolve_test_path(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}
