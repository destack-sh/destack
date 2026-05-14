use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use parking_lot::Mutex;

use destack_heap::{HeapReference, Root, RootSink, SharedHeapReference};

use crate::runtime::WorkerId;

/// One shared heap mark-root publication epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct SharedRootEpoch(u64);

impl SharedRootEpoch {
    /// Return the next shared root publication epoch.
    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Heap roots copied out for runtime coordination.
#[derive(Debug, Default)]
pub struct RootSet {
    /// Worker heap roots.
    heap: Vec<HeapReference>,
    /// Runtime heap roots.
    shared_heap: Vec<SharedHeapReference>,
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        Self {
            heap: Vec::new(),
            shared_heap: Vec::new(),
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
    pub fn push_shared_heap(&mut self, reference: SharedHeapReference) {
        if reference.is_null() {
            return;
        }

        self.shared_heap.push(reference);
    }

    /// Return worker heap roots.
    pub fn heap(&self) -> &[HeapReference] {
        self.heap.as_slice()
    }

    /// Return runtime heap roots.
    pub fn shared_heap(&self) -> &[SharedHeapReference] {
        self.shared_heap.as_slice()
    }
}

impl RootSink for RootSet {
    fn push(&mut self, root: Root) {
        match root {
            Root::HeapReference(reference) => Self::push_heap(self, reference),
            Root::SharedHeapReference(reference) => Self::push_shared_heap(self, reference),
        }
    }
}

/// Shared root set state for one mark cycle.
#[derive(Debug, Default)]
struct SharedRootSetState {
    /// Active shared mark-root publication epoch.
    epoch: SharedRootEpoch,
    /// Whether the root set is accepting active-cycle publications.
    is_active: bool,
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

/// Shared heap roots published by runtime workers during one mark cycle.
#[derive(Debug, Default)]
pub(crate) struct SharedRootSet {
    /// Shared root state protected as one coherent unit.
    state: Mutex<SharedRootSetState>,
    /// Whether shared mark termination is waiting on worker publication.
    termination_requested: AtomicBool,
    /// Current bounded local-to-shared edge scan work per worker.
    edge_scan_work_bytes: AtomicUsize,
}

impl SharedRootSet {
    /// Begin root publication for one shared mark cycle.
    pub(crate) fn begin_mark(
        &self,
        workers: impl IntoIterator<Item = WorkerId>,
        edge_scan_work_bytes: usize,
    ) -> SharedRootEpoch {
        let workers = workers.into_iter().collect::<BTreeSet<_>>();
        let mut state = self.state.lock();

        // epoch
        state.epoch = state.epoch.next();
        state.is_active = true;

        // root ownership
        state.direct_roots.clear();
        state.edge_roots.clear();
        state.direct_pending = workers.clone();
        state.edge_pending = workers;

        // cached snapshot
        state.roots = Arc::from(Vec::new());
        state.is_dirty = false;

        // publication state
        self.termination_requested.store(false, Ordering::Release);
        self.edge_scan_work_bytes
            .store(edge_scan_work_bytes, Ordering::Release);

        state.epoch
    }

    /// Finish root publication for one shared mark cycle.
    pub(crate) fn finish_mark(&self) {
        let mut state = self.state.lock();

        // root ownership
        state.is_active = false;
        state.direct_roots.clear();
        state.edge_roots.clear();
        state.direct_pending.clear();
        state.edge_pending.clear();

        // cached snapshot
        state.roots = Arc::from(Vec::new());
        state.is_dirty = false;

        // publication state
        self.termination_requested.store(false, Ordering::Release);
    }

    /// Join one worker to the active shared mark publication.
    pub(crate) fn join_mark(&self, worker_id: WorkerId) {
        let mut state = self.state.lock();
        if !state.is_active {
            return;
        }

        state.direct_pending.insert(worker_id);
        state.edge_pending.insert(worker_id);
    }

    /// Return the active shared mark-root publication epoch.
    pub(crate) fn active_epoch(&self) -> Option<SharedRootEpoch> {
        let state = self.state.lock();

        state.is_active.then_some(state.epoch)
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

    /// Return the current shared edge-scan work.
    pub(crate) fn edge_scan_work_bytes(&self) -> usize {
        self.edge_scan_work_bytes.load(Ordering::Acquire)
    }

    /// Set the current shared edge-scan work.
    pub(crate) fn set_edge_scan_work_bytes(&self, work_bytes: usize) {
        self.edge_scan_work_bytes
            .store(work_bytes, Ordering::Release);
    }

    /// Publish discovered shared edges into the runtime root state.
    pub(crate) fn push_edge_roots(
        &self,
        epoch: SharedRootEpoch,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        if roots.is_empty() {
            return;
        }

        let mut state = self.state.lock();
        if !state.is_active || state.epoch != epoch {
            return;
        }

        // append discovered edges for this worker
        state
            .edge_roots
            .entry(worker_id)
            .or_default()
            .extend_from_slice(roots);
        state.is_dirty = true;
    }

    /// Record one worker as pending one shared direct-root scan.
    pub(crate) fn queue_root_scan(&self, worker_id: WorkerId) {
        let mut state = self.state.lock();
        if state.is_active {
            state.direct_pending.insert(worker_id);
        }
    }

    /// Return the active epoch when one worker owes one direct-root scan.
    pub(crate) fn pending_root_epoch(&self, worker_id: WorkerId) -> Option<SharedRootEpoch> {
        let state = self.state.lock();
        if state.is_active && state.direct_pending.contains(&worker_id) {
            return Some(state.epoch);
        }

        None
    }

    /// Replace the cached direct roots for one worker.
    pub(crate) fn replace_direct_roots(
        &self,
        epoch: SharedRootEpoch,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        let mut state = self.state.lock();
        if !state.is_active || state.epoch != epoch {
            return;
        }

        // replace direct roots atomically for this worker
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

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_edge_scan(&self, epoch: SharedRootEpoch, worker_id: WorkerId) {
        let mut state = self.state.lock();
        if state.is_active && state.epoch == epoch {
            state.edge_pending.remove(&worker_id);
        }
    }

    /// Rebuild the combined shared-reference buffer when it is dirty.
    pub(crate) fn roots_snapshot(&self) -> Arc<[SharedHeapReference]> {
        let mut state = self.state.lock();
        if !state.is_dirty {
            return state.roots.clone();
        }

        let mut roots = Vec::new();

        // direct roots
        for worker_roots in state.direct_roots.values() {
            roots.extend(worker_roots.iter().copied());
        }

        // local-to-shared edges
        for worker_roots in state.edge_roots.values() {
            roots.extend(worker_roots.iter().copied());
        }

        // deterministic compact snapshot
        roots.sort_unstable_by_key(|reference| reference.bits());
        roots.dedup_by_key(|reference| reference.bits());
        state.roots = Arc::from(roots);
        state.is_dirty = false;

        state.roots.clone()
    }
}
