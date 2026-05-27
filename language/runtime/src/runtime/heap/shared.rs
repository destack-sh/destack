use std::sync::Arc;

use destack_heap::{self as heap, SharedHeapReference};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::heap::{SharedRootEpoch, SharedRootSet, resolve_shared_heap_options};
use crate::runtime::{SharedCollection, SharedCollector, WorkerId};
use destack_workspace::RuntimeOptions;

/// Runtime-owned shared heap and collection state.
#[derive(Debug)]
pub(crate) struct SharedHeap {
    /// History-owned allocator backing runtime and worker heaps.
    pub(crate) allocator: Arc<heap::Allocator>,
    /// Shared heap visible to every worker in this runtime.
    pub(crate) heap: Arc<heap::SharedHeap>,
    /// Shared heap roots published by workers for the active mark cycle.
    roots: Arc<SharedRootSet>,
    /// Runtime-owned collection state for this shared heap.
    collection: Arc<SharedCollection>,
    /// History-owned collection scheduler.
    collector: Arc<SharedCollector>,
}

impl SharedHeap {
    /// Create runtime-owned shared heap state from runtime options.
    pub(crate) fn new(
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
        options: &RuntimeOptions,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::with_allocator_limits_and_options(
                allocator.clone(),
                shared_heap_options.limits,
                shared_heap_options.options,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_heap(allocator, shared, collector))
    }

    /// Restore runtime-owned shared heap state from a snapshot.
    pub(crate) fn from_snapshot(
        snapshot: &heap::SharedHeapSnapshot,
        options: &RuntimeOptions,
        allocator: Arc<heap::Allocator>,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::from_snapshot_with_allocator(
                snapshot,
                shared_heap_options.limits,
                allocator.clone(),
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_heap(allocator, shared, collector))
    }

    /// Fork runtime-owned shared heap state for one child world.
    pub(crate) fn fork(&self, collector: Arc<SharedCollector>) -> RuntimeResult<Self> {
        let shared = Arc::new(self.heap.fork().map_err(Box::<RuntimeError>::from)?);

        Ok(Self::from_heap(self.allocator.clone(), shared, collector))
    }

    /// Capture the shared heap snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<heap::SharedHeapSnapshot> {
        self.heap
            .image()
            .and_then(|image| image.snapshot())
            .map_err(Box::<RuntimeError>::from)
    }

    /// Borrow the shared root set.
    pub(crate) fn roots(&self) -> &SharedRootSet {
        &self.roots
    }

    /// Register one shared GC worker.
    pub(crate) fn register_collector_worker(&self) -> heap::SharedGcWorker {
        self.heap.register_collector_worker()
    }

    /// Return whether shared collection work is active.
    pub(crate) fn collection_busy(&self) -> bool {
        self.collection.is_busy()
    }

    /// Return one shared collector failure if one was recorded.
    pub(crate) fn take_collection_failure(&self) -> Option<Box<RuntimeError>> {
        self.collection.take_failure()
    }

    /// Suspend shared GC and wait for in-flight work to drain.
    pub(crate) fn quiesce(&self) {
        self.collection.quiesce();
    }

    /// Resume shared GC after one quiescent operation.
    pub(crate) fn resume(&self) {
        self.collection.resume();

        if self.collector.mode().is_concurrent()
            && self.heap.gc_phase() != heap::SharedGcPhase::Idle
        {
            self.collector.wake(&self.collection);
        }
    }

    /// Wake concurrent shared GC work.
    pub(crate) fn wake(&self) {
        self.collector.wake(&self.collection);
    }

    /// Return whether the shared heap is currently marking.
    pub(crate) fn is_marking(&self) -> bool {
        self.heap.gc_phase() == heap::SharedGcPhase::Mark
    }

    /// Return the bounded shared local-edge scan work for one worker tick.
    pub(crate) fn edge_scan_work_bytes(&self) -> usize {
        self.roots.edge_scan_work_bytes()
    }

    /// Publish discovered shared edges into the runtime root state.
    pub(crate) fn push_edge_roots(&self, worker_id: WorkerId, roots: &[SharedHeapReference]) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.push_edge_roots(epoch, worker_id, roots);
        self.wake();
    }

    /// Return the active epoch when this worker owes one direct shared-root publication.
    pub(crate) fn pending_root_epoch(&self, worker_id: WorkerId) -> Option<SharedRootEpoch> {
        self.roots.pending_root_epoch(worker_id)
    }

    /// Replace the direct shared roots cached for one worker.
    pub(crate) fn replace_direct_roots(
        &self,
        epoch: SharedRootEpoch,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.roots.replace_direct_roots(epoch, worker_id, roots);
        self.wake();
    }

    /// Queue one worker for one later shared direct-root rescan.
    pub(crate) fn queue_root_scan(&self, worker_id: WorkerId) {
        self.roots.queue_root_scan(worker_id);
    }

    /// Join one worker to an active shared mark cycle.
    pub(crate) fn join_mark(&self, worker_id: WorkerId) {
        self.roots.join_mark(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_edge_scan(&self, worker_id: WorkerId) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.leave_edge_scan(epoch, worker_id);
        self.wake();
    }

    /// Return whether shared mark termination is waiting on worker publication.
    pub(crate) fn is_terminating(&self) -> bool {
        self.roots.termination_requested()
    }

    /// Remove one worker from shared root publication state.
    pub(crate) fn remove_worker(&self, worker_id: WorkerId) {
        self.roots.remove_worker(worker_id);
        self.wake();
    }

    /// Return whether concurrent collection is enabled.
    pub(crate) fn is_concurrent(&self) -> bool {
        self.collector.mode().is_concurrent()
    }

    /// Build state around one already-created shared heap.
    fn from_heap(
        allocator: Arc<heap::Allocator>,
        shared: Arc<heap::SharedHeap>,
        collector: Arc<SharedCollector>,
    ) -> Self {
        let roots = Arc::new(SharedRootSet::default());
        let collection = SharedCollection::new(shared.clone(), roots.clone());

        Self {
            allocator,
            heap: shared,
            roots,
            collection,
            collector,
        }
    }
}
