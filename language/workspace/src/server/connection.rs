use std::collections::HashMap;
use std::path::PathBuf;

use crate::protocol::{ProtocolLimits, RootId};

/// Open root associated with one protocol handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct OpenRoot {
    /// Opened root path.
    pub(super) root: PathBuf,
}

/// Mutable state for one protocol connection.
#[derive(Debug)]
pub(super) struct Connection {
    /// Whether the handshake has completed.
    pub(super) is_ready: bool,
    /// Current protocol limits for this connection.
    pub(super) limits: ProtocolLimits,

    /// Next root handle id to allocate.
    next_root_handle: u64,
    /// Open roots keyed by handle id.
    root_by_handle: HashMap<RootId, OpenRoot>,
    /// Handles keyed by open root.
    handle_by_root: HashMap<OpenRoot, RootId>,

    /// Whether a shutdown was requested.
    pub(super) shutting_down: bool,
}

impl Connection {
    /// Create fresh connection state.
    pub(super) fn new(limits: ProtocolLimits) -> Self {
        Self {
            is_ready: false,
            limits,
            next_root_handle: 1,
            root_by_handle: HashMap::new(),
            handle_by_root: HashMap::new(),
            shutting_down: false,
        }
    }

    /// Return the handle for an open root if it is already registered.
    pub(super) fn handle(&self, root: &OpenRoot) -> Option<RootId> {
        self.handle_by_root.get(root).copied()
    }

    /// Insert one root handle.
    pub(super) fn insert_root(&mut self, entry: OpenRoot) -> RootId {
        let handle = self.allocate_root_handle();
        self.handle_by_root.insert(entry.clone(), handle);
        self.root_by_handle.insert(handle, entry);

        handle
    }

    /// Remove one root handle.
    pub(super) fn remove_root(&mut self, handle: RootId) -> Option<OpenRoot> {
        let entry = self.root_by_handle.remove(&handle)?;
        self.handle_by_root.remove(&entry);

        Some(entry)
    }

    /// Return the open root for one handle.
    pub(super) fn root(&self, handle: RootId) -> Option<OpenRoot> {
        self.root_by_handle.get(&handle).cloned()
    }

    /// Drain all root handles.
    pub(super) fn drain_roots(&mut self) -> Vec<OpenRoot> {
        self.handle_by_root.clear();

        self.root_by_handle
            .drain()
            .map(|(_, entry)| entry)
            .collect()
    }

    /// Allocate a root handle id.
    fn allocate_root_handle(&mut self) -> RootId {
        let handle = RootId::new(self.next_root_handle);
        self.next_root_handle += 1;

        handle
    }
}
