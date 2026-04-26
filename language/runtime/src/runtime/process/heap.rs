use std::sync::Arc;

use destack_heap::{self as heap, SharedHeapReference};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{SharedMarkRoots, resolve_shared_heap_options};
use crate::runtime::{Collection, Collector, WorkerId};
use destack_workspace::RuntimeOptions;

/// Runtime-owned shared heap and collection state.
#[derive(Debug)]
pub(crate) struct RuntimeSharedHeap {
    /// Shared heap visible to every worker in this runtime.
    shared: Arc<heap::SharedHeap>,
    /// Published roots for the active shared mark cycle.
    mark_roots: Arc<SharedMarkRoots>,
    /// Shared heap collection state.
    collection: Arc<Collection>,
    /// Lineage-owned collection scheduler.
    collector: Arc<Collector>,
}

impl RuntimeSharedHeap {
    /// Create runtime-owned shared heap state from runtime options.
    pub(crate) fn new(
        allocator: Arc<heap::Allocator>,
        collector: Arc<Collector>,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::with_allocator_limits_and_options(
                allocator,
                shared_heap_options.limits,
                shared_heap_options.options,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_shared(shared, collector))
    }

    /// Restore runtime-owned shared heap state from a snapshot.
    pub(crate) fn from_snapshot(
        snapshot: &heap::SharedHeapSnapshot,
        options: &RuntimeOptions,
        collector: Arc<Collector>,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::from_snapshot_with_limits(snapshot, shared_heap_options.limits)
                .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_shared(shared, collector))
    }

    /// Fork runtime-owned shared heap state for one child world.
    pub(crate) fn fork(&self, collector: Arc<Collector>) -> RuntimeResult<Self> {
        let shared = Arc::new(self.shared.fork().map_err(Box::<RuntimeError>::from)?);

        Ok(Self::from_shared(shared, collector))
    }

    /// Capture the shared heap snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<heap::SharedHeapSnapshot> {
        self.shared
            .image()
            .and_then(|image| image.snapshot())
            .map_err(Box::<RuntimeError>::from)
    }

    /// Borrow the shared heap.
    pub(crate) fn shared(&self) -> &heap::SharedHeap {
        &self.shared
    }

    /// Borrow the shared mark roots.
    pub(crate) fn mark_roots(&self) -> &SharedMarkRoots {
        &self.mark_roots
    }

    /// Borrow the shared collection state.
    pub(crate) fn collection(&self) -> &Arc<Collection> {
        &self.collection
    }

    /// Return one shared GC worker handle.
    pub(crate) fn worker(&self, worker_id: WorkerId) -> RuntimeResult<heap::SharedGcWorker> {
        let worker_index = usize::try_from(worker_id.0).map_err(|_| RuntimeError::Internal {
            message: format!("worker id {} cannot index shared gc work", worker_id.0),
        })?;

        Ok(self.shared.gc_worker(worker_index))
    }

    /// Suspend shared GC and wait for in-flight work to drain.
    pub(crate) fn quiesce(&self) {
        self.collection.quiesce();
    }

    /// Resume shared GC after one quiescent operation.
    pub(crate) fn resume(&self) {
        self.collection.resume();

        if self.collector.mode().is_concurrent()
            && self.shared.gc_phase() != heap::SharedGcPhase::Idle
        {
            self.collector.wake(&self.collection);
        }
    }

    /// Wake concurrent shared GC work.
    pub(crate) fn wake(&self) {
        self.collector.wake(&self.collection);
    }

    /// Return whether the shared heap is currently marking.
    pub(crate) fn shared_gc_marking(&self) -> bool {
        self.shared.gc_phase() == heap::SharedGcPhase::Mark
    }

    /// Return the bounded shared local-edge scan budget for one worker tick.
    pub(crate) fn shared_edge_scan_tick_budget(&self) -> usize {
        self.mark_roots.edge_scan_budget()
    }

    /// Publish discovered shared edges into the runtime root state.
    pub(crate) fn push_shared_edge_roots(
        &self,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        self.mark_roots.push_edge_roots(worker_id, roots);
        self.wake();
    }

    /// Return whether this worker still owes one direct shared-root publication.
    pub(crate) fn is_shared_root_scan_pending(&self, worker_id: WorkerId) -> bool {
        self.mark_roots.is_root_scan_pending(worker_id)
    }

    /// Replace the direct shared roots cached for one worker.
    pub(crate) fn replace_shared_direct_roots(
        &self,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.mark_roots.replace_direct_roots(worker_id, roots);
        self.wake();
    }

    /// Queue one worker for one later shared direct-root rescan.
    pub(crate) fn queue_shared_root_scan(&self, worker_id: WorkerId) {
        self.mark_roots.queue_root_scan(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_shared_edge_scan(&self, worker_id: WorkerId) {
        self.mark_roots.leave_edge_scan(worker_id);
        self.wake();
    }

    /// Record one worker as participating in the active shared local-edge pass.
    pub(crate) fn join_shared_edge_scan(&self, worker_id: WorkerId) {
        self.mark_roots.join_edge_scan(worker_id);
    }

    /// Return whether shared mark termination is waiting on worker publication.
    pub(crate) fn shared_gc_terminating(&self) -> bool {
        self.mark_roots.termination_requested()
    }

    /// Remove one worker from shared root publication state.
    pub(crate) fn remove_worker(&self, worker_id: WorkerId) {
        self.mark_roots.remove_worker(worker_id);
        self.wake();
    }

    /// Return whether concurrent collection is enabled.
    pub(crate) fn is_concurrent(&self) -> bool {
        self.collector.mode().is_concurrent()
    }

    /// Build state around one already-created shared heap.
    fn from_shared(shared: Arc<heap::SharedHeap>, collector: Arc<Collector>) -> Self {
        let mark_roots = Arc::new(SharedMarkRoots::default());
        let collection = Collection::new(shared.clone(), mark_roots.clone());

        Self {
            shared,
            mark_roots,
            collection,
            collector,
        }
    }
}
