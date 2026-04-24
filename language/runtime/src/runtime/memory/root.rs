use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use parking_lot::{Mutex, RwLock};

use destack_heap::{HeapReference, SharedHeapReference};

use crate::runtime::WorkerId;

/// Collection of GC roots for a safepoint.
#[derive(Debug, Default)]
pub struct RootSet {
    /// Local heap roots for this collection.
    heap: Vec<HeapReference>,
    /// Shared heap roots for this collection.
    shared: Vec<SharedHeapReference>,
}

/// Cached shared root snapshot for one mark cycle.
#[derive(Debug, Default)]
struct RootSnapshot {
    /// The current combined shared-reference snapshot for GC work.
    roots: RwLock<Arc<[SharedHeapReference]>>,
    /// Whether the combined shared-reference buffer must be rebuilt.
    is_dirty: AtomicBool,
}

impl RootSnapshot {
    /// Mark the cached root snapshot dirty.
    fn mark_dirty(&self) {
        self.is_dirty.store(true, Ordering::Release);
    }

    /// Return the cached root snapshot, rebuilding it if needed.
    fn get(
        &self,
        direct: &Mutex<BTreeMap<WorkerId, Vec<SharedHeapReference>>>,
        edges: &EdgeBuffer,
    ) -> Arc<[SharedHeapReference]> {
        if self.is_dirty.load(Ordering::Acquire) {
            self.rebuild(direct, edges);
        }

        self.roots.read().clone()
    }

    /// Rebuild the root snapshot from direct roots and edge roots.
    fn rebuild(
        &self,
        direct: &Mutex<BTreeMap<WorkerId, Vec<SharedHeapReference>>>,
        edges: &EdgeBuffer,
    ) {
        let mut roots = Vec::new();

        for worker_roots in direct.lock().values() {
            roots.extend(worker_roots.iter().copied());
        }

        edges.rebuild_into(&mut roots);
        roots.sort_unstable_by_key(|reference| reference.bits());
        roots.dedup_by_key(|reference| reference.bits());
        *self.roots.write() = Arc::from(roots);
        self.is_dirty.store(false, Ordering::Release);
    }
}

/// Local-to-shared edge publication buffer.
#[derive(Debug, Default)]
struct EdgeBuffer {
    /// The published shared edge roots per worker.
    by_worker: Mutex<BTreeMap<WorkerId, Vec<SharedHeapReference>>>,
}

impl EdgeBuffer {
    /// Publish one batch of shared edge roots for one worker.
    fn push(&self, worker_id: WorkerId, roots: &[SharedHeapReference]) {
        if roots.is_empty() {
            return;
        }

        self.by_worker
            .lock()
            .entry(worker_id)
            .or_default()
            .extend_from_slice(roots);
    }

    /// Drain every shared edge root into one combined buffer.
    fn rebuild_into(&self, roots: &mut Vec<SharedHeapReference>) {
        for worker_roots in self.by_worker.lock().values() {
            roots.extend(worker_roots.iter().copied());
        }
    }

    /// Clear every published shared edge root.
    fn clear(&self) {
        self.by_worker.lock().clear();
    }
}

/// Worker scan state for one shared mark cycle.
#[derive(Debug, Default)]
struct MarkRootScan {
    /// Workers still participating in the active shared direct-root pass.
    direct_pending: Mutex<BTreeSet<WorkerId>>,
    /// Workers still participating in the active shared local-edge pass.
    edge_pending: Mutex<BTreeSet<WorkerId>>,
}

impl MarkRootScan {
    /// Clear the active shared direct-root pass.
    fn clear_direct(&self) {
        self.direct_pending.lock().clear();
    }

    /// Clear the active shared local-edge pass.
    fn clear_edge(&self) {
        self.edge_pending.lock().clear();
    }

    /// Return whether the active shared direct-root pass is drained.
    fn direct_idle(&self) -> bool {
        self.direct_pending.lock().is_empty()
    }

    /// Return whether the active shared local-edge pass is drained.
    fn edge_idle(&self) -> bool {
        self.edge_pending.lock().is_empty()
    }

    /// Record one worker as pending one shared direct-root scan.
    fn queue_direct(&self, worker_id: WorkerId) {
        self.direct_pending.lock().insert(worker_id);
    }

    /// Remove one worker from the active shared direct-root pass.
    fn leave_direct(&self, worker_id: WorkerId) {
        self.direct_pending.lock().remove(&worker_id);
    }

    /// Return whether one worker is still pending one shared direct-root scan.
    fn is_direct_pending(&self, worker_id: WorkerId) -> bool {
        self.direct_pending.lock().contains(&worker_id)
    }

    /// Record one worker as participating in the active shared local-edge pass.
    fn join_edge(&self, worker_id: WorkerId) {
        self.edge_pending.lock().insert(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    fn leave_edge(&self, worker_id: WorkerId) {
        self.edge_pending.lock().remove(&worker_id);
    }
}

/// Root publication state for one shared mark cycle.
#[derive(Debug, Default)]
pub(crate) struct MarkRootSet {
    /// Cached deduplicated root snapshot.
    snapshot: RootSnapshot,
    /// Cached direct shared roots for each worker.
    direct_roots: Mutex<BTreeMap<WorkerId, Vec<SharedHeapReference>>>,
    /// Shared edges discovered through local-to-shared edge scanning.
    edge_roots: EdgeBuffer,
    /// Worker scan state for this mark cycle.
    scan: MarkRootScan,
    /// Whether shared mark termination is waiting on worker publication.
    termination_requested: AtomicBool,
    /// Current bounded local-to-shared edge scan budget per worker step.
    edge_scan_budget: AtomicUsize,
}

impl MarkRootSet {
    /// Clear one active shared direct-root pass.
    pub(crate) fn clear_root_scan(&self) {
        self.scan.clear_direct();
    }

    /// Clear one active shared local-edge pass.
    pub(crate) fn clear_edge_scan(&self) {
        self.scan.clear_edge();
        self.edge_roots.clear();
        self.snapshot.mark_dirty();
    }

    /// Return whether the active shared direct-root pass is drained.
    pub(crate) fn root_scan_idle(&self) -> bool {
        self.scan.direct_idle()
    }

    /// Return whether the active shared local-edge pass is drained.
    pub(crate) fn edge_scan_idle(&self) -> bool {
        self.scan.edge_idle()
    }

    /// Return whether both shared root passes are currently drained.
    pub(crate) fn roots_complete(&self) -> bool {
        self.root_scan_idle() && self.edge_scan_idle()
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
        self.edge_roots.push(worker_id, roots);
        self.snapshot.mark_dirty();
    }

    /// Record one worker as pending one shared direct-root scan.
    pub(crate) fn queue_root_scan(&self, worker_id: WorkerId) {
        self.scan.queue_direct(worker_id);
    }

    /// Remove one worker from the active shared direct-root pass.
    pub(crate) fn leave_root_scan(&self, worker_id: WorkerId) {
        self.scan.leave_direct(worker_id);
    }

    /// Return whether one worker is still pending one shared direct-root scan.
    pub(crate) fn is_root_scan_pending(&self, worker_id: WorkerId) -> bool {
        self.scan.is_direct_pending(worker_id)
    }

    /// Replace the cached direct roots for one worker.
    pub(crate) fn replace_direct_roots(
        &self,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.direct_roots.lock().insert(worker_id, roots);
        self.scan.leave_direct(worker_id);
        self.snapshot.mark_dirty();
    }

    /// Remove one worker from cached direct roots.
    pub(crate) fn remove_direct_roots(&self, worker_id: WorkerId) {
        self.direct_roots.lock().remove(&worker_id);
        self.snapshot.mark_dirty();
    }

    /// Record one worker as participating in the active shared local-edge pass.
    pub(crate) fn join_edge_scan(&self, worker_id: WorkerId) {
        self.scan.join_edge(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_edge_scan(&self, worker_id: WorkerId) {
        self.scan.leave_edge(worker_id);
    }

    /// Clear cached direct roots for one new cycle.
    pub(crate) fn clear_direct_roots(&self) {
        self.direct_roots.lock().clear();
        self.snapshot.mark_dirty();
    }

    /// Rebuild the combined shared-reference buffer when it is dirty.
    pub(crate) fn roots_snapshot(&self) -> Arc<[SharedHeapReference]> {
        self.snapshot.get(&self.direct_roots, &self.edge_roots)
    }
}

impl RootSet {
    /// Create an empty root set.
    pub fn new() -> Self {
        Self {
            heap: Vec::new(),
            shared: Vec::new(),
        }
    }

    /// Push one local heap root reference.
    pub fn push_heap(&mut self, reference: HeapReference) {
        self.heap.push(reference);
    }

    /// Push one shared heap root reference.
    pub fn push_shared(&mut self, reference: SharedHeapReference) {
        self.shared.push(reference);
    }

    /// Extend this root set with local heap references.
    pub fn extend_heap(&mut self, references: impl IntoIterator<Item = HeapReference>) {
        self.heap.extend(references);
    }

    /// Extend this root set with shared heap references.
    pub fn extend_shared(&mut self, references: impl IntoIterator<Item = SharedHeapReference>) {
        self.shared.extend(references);
    }

    /// Extend this root set with another root set.
    pub fn extend(&mut self, roots: Self) {
        self.heap.extend(roots.heap);
        self.shared.extend(roots.shared);
    }

    /// Return the local heap roots for this collection.
    pub fn heap(&self) -> &[HeapReference] {
        self.heap.as_slice()
    }

    /// Return the shared heap roots for this collection.
    pub fn shared(&self) -> &[SharedHeapReference] {
        self.shared.as_slice()
    }
}

/// Root destination for one scan.
#[derive(Debug)]
pub enum RootVisitor<'a> {
    /// Record both local and shared roots.
    All(&'a mut RootSet),
    /// Record only local heap roots.
    Heap(&'a mut Vec<HeapReference>),
    /// Record only shared heap roots.
    SharedHeap(&'a mut Vec<SharedHeapReference>),
}

impl RootVisitor<'_> {
    /// Record one local heap root.
    pub fn push_heap(&mut self, reference: HeapReference) {
        match self {
            Self::All(roots) => roots.push_heap(reference),
            Self::Heap(roots) => roots.push(reference),
            Self::SharedHeap(_) => {}
        }
    }

    /// Record one shared heap root.
    pub fn push_shared(&mut self, reference: SharedHeapReference) {
        match self {
            Self::All(roots) => roots.push_shared(reference),
            Self::Heap(_) => {}
            Self::SharedHeap(roots) => roots.push(reference),
        }
    }
}

/// Visit GC roots for a safepoint collection.
pub trait RootProvider: Send + Sync {
    /// Visit roots through the provided sink.
    fn visit_roots(&self, roots: &mut RootVisitor<'_>);
}
