use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::Watch;

use super::{ProtocolLimits, RepositoryId, RootHandleId};

/// Open root associated with one protocol handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OpenRoot {
    /// Workspace root.
    pub(super) workspace: PathBuf,
    /// Opened root path.
    pub(super) root: PathBuf,
}

/// Mutable state for one protocol connection.
#[derive(Debug)]
pub(super) struct Connection {
    /// Current session id for this connection.
    pub(super) session_id: Option<RepositoryId>,
    /// The negotiated protocol limits.
    pub(super) negotiated_limits: Option<ProtocolLimits>,
    /// Next root handle id to allocate.
    next_root_handle: u64,
    /// Open roots keyed by handle id.
    root_by_handle: HashMap<RootHandleId, OpenRoot>,
    /// Handles keyed by open root.
    handle_by_root: HashMap<OpenRoot, RootHandleId>,
    /// Watch subscriptions keyed by root handle.
    watch_by_handle: HashMap<RootHandleId, Arc<Watch>>,
    /// Whether a shutdown was requested.
    pub(super) shutting_down: bool,
}

impl Connection {
    /// Create fresh connection state.
    pub(super) fn new() -> Self {
        Self {
            session_id: None,
            negotiated_limits: None,
            next_root_handle: 1,
            root_by_handle: HashMap::new(),
            handle_by_root: HashMap::new(),
            watch_by_handle: HashMap::new(),
            shutting_down: false,
        }
    }

    /// Return the handle for an open root if it is already registered.
    pub(super) fn handle(&self, root: &OpenRoot) -> Option<RootHandleId> {
        self.handle_by_root.get(root).copied()
    }

    /// Insert one root handle.
    pub(super) fn insert_root(&mut self, entry: OpenRoot) -> RootHandleId {
        let handle = self.allocate_root_handle();
        self.handle_by_root.insert(entry.clone(), handle);
        self.root_by_handle.insert(handle, entry);

        handle
    }

    /// Remove one root handle.
    pub(super) fn remove_root(&mut self, handle: RootHandleId) -> Option<OpenRoot> {
        let entry = self.root_by_handle.remove(&handle)?;
        self.handle_by_root.remove(&entry);
        self.stop_watch(handle);

        Some(entry)
    }

    /// Return the open root for one handle.
    pub(super) fn root(&self, handle: RootHandleId) -> Option<OpenRoot> {
        self.root_by_handle.get(&handle).cloned()
    }

    /// Drain all root handles.
    pub(super) fn drain_roots(&mut self) -> Vec<OpenRoot> {
        self.handle_by_root.clear();
        for (_, watch) in self.watch_by_handle.drain() {
            watch.stop();
        }

        self.root_by_handle
            .drain()
            .map(|(_, entry)| entry)
            .collect()
    }

    /// Start watching one root handle.
    pub(super) fn start_watch(&mut self, handle: RootHandleId, watch: Arc<Watch>) {
        if let Some(previous) = self.watch_by_handle.insert(handle, watch) {
            previous.stop();
        }
    }

    /// Return one active watch.
    pub(super) fn watch(&self, handle: RootHandleId) -> Option<Arc<Watch>> {
        self.watch_by_handle.get(&handle).cloned()
    }

    /// Stop watching one root handle.
    pub(super) fn stop_watch(&mut self, handle: RootHandleId) -> bool {
        let Some(watch) = self.watch_by_handle.remove(&handle) else {
            return false;
        };
        watch.stop();

        true
    }

    /// Allocate a root handle id.
    fn allocate_root_handle(&mut self) -> RootHandleId {
        let handle = RootHandleId::new(self.next_root_handle);
        self.next_root_handle += 1;

        handle
    }
}
