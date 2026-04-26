use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use parking_lot::Mutex;

use destack_heap::{HeapReference, SharedHeapReference};

use crate::runtime::WorkerId;

/// Heap roots copied out for runtime coordination.
#[derive(Debug, Default)]
pub struct RootSet {
    /// Worker heap roots.
    heap: Vec<HeapReference>,
    /// Runtime heap roots.
    shared: Vec<SharedHeapReference>,
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        Self {
            heap: Vec::new(),
            shared: Vec::new(),
        }
    }

    /// Record one worker heap root.
    pub fn push_heap(&mut self, reference: HeapReference) {
        if reference.is_null() {
            return;
        }

        self.heap.push(reference);
    }

    /// Record one runtime heap root.
    pub fn push_shared(&mut self, reference: SharedHeapReference) {
        if reference.is_null() {
            return;
        }

        self.shared.push(reference);
    }

    /// Return worker heap roots.
    pub fn heap(&self) -> &[HeapReference] {
        self.heap.as_slice()
    }

    /// Return runtime heap roots.
    pub fn shared(&self) -> &[SharedHeapReference] {
        self.shared.as_slice()
    }
}

/// Root sink for copied runtime roots.
#[derive(Debug)]
pub enum RootVisitor<'a> {
    /// Record both worker and runtime heap roots.
    All(&'a mut RootSet),
    /// Record only worker heap roots.
    Heap(&'a mut Vec<HeapReference>),
    /// Record only runtime heap roots.
    SharedHeap(&'a mut Vec<SharedHeapReference>),
}

impl RootVisitor<'_> {
    /// Record one worker heap root.
    pub fn push_heap(&mut self, reference: HeapReference) {
        if reference.is_null() {
            return;
        }

        match self {
            Self::All(roots) => roots.push_heap(reference),
            Self::Heap(roots) => roots.push(reference),
            Self::SharedHeap(_) => {}
        }
    }

    /// Record one runtime heap root.
    pub fn push_shared(&mut self, reference: SharedHeapReference) {
        if reference.is_null() {
            return;
        }

        match self {
            Self::All(roots) => roots.push_shared(reference),
            Self::Heap(_) => {}
            Self::SharedHeap(roots) => roots.push(reference),
        }
    }
}

/// Shared root publication state for one mark cycle.
#[derive(Debug, Default)]
struct SharedRootState {
    /// Cached direct shared roots for each worker.
    direct_roots: BTreeMap<WorkerId, Vec<SharedHeapReference>>,
    /// Shared edges discovered through local-to-shared edge scanning.
    edge_roots: BTreeMap<WorkerId, Vec<SharedHeapReference>>,
    /// Workers still participating in the active shared direct-root pass.
    direct_pending: BTreeSet<WorkerId>,
    /// Workers still participating in the active shared local-edge pass.
    edge_pending: BTreeSet<WorkerId>,
    /// The current combined shared-reference snapshot for GC work.
    roots: Arc<[SharedHeapReference]>,
    /// Whether the combined shared-reference buffer must be rebuilt.
    is_dirty: bool,
}

/// Root publication state for one shared mark cycle.
#[derive(Debug, Default)]
pub(crate) struct SharedMarkRoots {
    /// Shared root state protected as one coherent unit.
    state: Mutex<SharedRootState>,
    /// Whether shared mark termination is waiting on worker publication.
    termination_requested: AtomicBool,
    /// Current bounded local-to-shared edge scan budget per worker step.
    edge_scan_budget: AtomicUsize,
}

impl SharedMarkRoots {
    /// Clear one active shared direct-root pass.
    pub(crate) fn clear_root_scan(&self) {
        self.state.lock().direct_pending.clear();
    }

    /// Clear one active shared local-edge pass.
    pub(crate) fn clear_edge_scan(&self) {
        let mut state = self.state.lock();
        state.edge_pending.clear();
        state.edge_roots.clear();
        state.is_dirty = true;
    }

    /// Return whether the active shared direct-root pass is drained.
    pub(crate) fn root_scan_idle(&self) -> bool {
        self.state.lock().direct_pending.is_empty()
    }

    /// Return whether the active shared local-edge pass is drained.
    pub(crate) fn edge_scan_idle(&self) -> bool {
        self.state.lock().edge_pending.is_empty()
    }

    /// Return whether both shared root passes are currently drained.
    pub(crate) fn roots_complete(&self) -> bool {
        let state = self.state.lock();

        state.direct_pending.is_empty() && state.edge_pending.is_empty()
    }

    /// Return whether shared mark termination is waiting on worker publication.
    pub(crate) fn termination_requested(&self) -> bool {
        self.termination_requested.load(Ordering::Acquire)
    }

    /// Request shared mark termination publication from all pending workers.
    pub(crate) fn request_termination(&self) {
        self.termination_requested.store(true, Ordering::Release);
    }

    /// Clear one outstanding shared mark termination request.
    pub(crate) fn clear_termination(&self) {
        self.termination_requested.store(false, Ordering::Release);
    }

    /// Return the current shared edge-scan budget.
    pub(crate) fn edge_scan_budget(&self) -> usize {
        self.edge_scan_budget.load(Ordering::Acquire)
    }

    /// Set the current shared edge-scan budget.
    pub(crate) fn set_edge_scan_budget(&self, budget: usize) {
        self.edge_scan_budget.store(budget, Ordering::Release);
    }

    /// Publish discovered shared edges into the world root state.
    pub(crate) fn push_edge_roots(&self, worker_id: WorkerId, roots: &[SharedHeapReference]) {
        if roots.is_empty() {
            return;
        }

        let mut state = self.state.lock();
        state
            .edge_roots
            .entry(worker_id)
            .or_default()
            .extend_from_slice(roots);
        state.is_dirty = true;
    }

    /// Record one worker as pending one shared direct-root scan.
    pub(crate) fn queue_root_scan(&self, worker_id: WorkerId) {
        self.state.lock().direct_pending.insert(worker_id);
    }

    /// Return whether one worker is still pending one shared direct-root scan.
    pub(crate) fn is_root_scan_pending(&self, worker_id: WorkerId) -> bool {
        self.state.lock().direct_pending.contains(&worker_id)
    }

    /// Replace the cached direct roots for one worker.
    pub(crate) fn replace_direct_roots(
        &self,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        let mut state = self.state.lock();
        state.direct_roots.insert(worker_id, roots);
        state.direct_pending.remove(&worker_id);
        state.is_dirty = true;
    }

    /// Remove one worker from shared mark state.
    pub(crate) fn remove_worker(&self, worker_id: WorkerId) {
        let mut state = self.state.lock();
        state.direct_roots.remove(&worker_id);
        state.edge_roots.remove(&worker_id);
        state.direct_pending.remove(&worker_id);
        state.edge_pending.remove(&worker_id);
        state.is_dirty = true;
    }

    /// Record one worker as participating in the active shared local-edge pass.
    pub(crate) fn join_edge_scan(&self, worker_id: WorkerId) {
        self.state.lock().edge_pending.insert(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_edge_scan(&self, worker_id: WorkerId) {
        self.state.lock().edge_pending.remove(&worker_id);
    }

    /// Clear cached direct roots for one new cycle.
    pub(crate) fn clear_direct_roots(&self) {
        let mut state = self.state.lock();
        state.direct_roots.clear();
        state.is_dirty = true;
    }

    /// Rebuild the combined shared-reference buffer when it is dirty.
    pub(crate) fn roots_snapshot(&self) -> Arc<[SharedHeapReference]> {
        let mut state = self.state.lock();
        if !state.is_dirty {
            return state.roots.clone();
        }

        let mut roots = Vec::new();

        for worker_roots in state.direct_roots.values() {
            roots.extend(worker_roots.iter().copied());
        }

        for worker_roots in state.edge_roots.values() {
            roots.extend(worker_roots.iter().copied());
        }

        roots.sort_unstable_by_key(|reference| reference.bits());
        roots.dedup_by_key(|reference| reference.bits());
        state.roots = Arc::from(roots);
        state.is_dirty = false;

        state.roots.clone()
    }
}
