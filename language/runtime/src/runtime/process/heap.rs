use std::sync::Arc;

use destack_heap::{self as heap, SharedHeapReference};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{SharedRootEpoch, SharedRootSet, resolve_shared_heap_options};
use crate::runtime::{Collector, CollectorWork, WorkerId};
use destack_workspace::RuntimeOptions;

/// Runtime-owned shared heap and collection state.
#[derive(Debug)]
pub(crate) struct RuntimeSharedHeap {
    /// Lineage-owned allocator backing runtime and worker heaps.
    allocator: Arc<heap::Allocator>,
    /// Shared heap visible to every worker in this runtime.
    shared: Arc<heap::SharedHeap>,
    /// Shared heap roots published by workers for the active mark cycle.
    roots: Arc<SharedRootSet>,
    /// Runtime-owned collector work for this shared heap.
    collector_work: Arc<CollectorWork>,
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
                allocator.clone(),
                shared_heap_options.limits,
                shared_heap_options.options,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_shared(allocator, shared, collector))
    }

    /// Restore runtime-owned shared heap state from a snapshot.
    pub(crate) fn from_snapshot(
        snapshot: &heap::SharedHeapSnapshot,
        options: &RuntimeOptions,
        allocator: Arc<heap::Allocator>,
        collector: Arc<Collector>,
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

        Ok(Self::from_shared(allocator, shared, collector))
    }

    /// Fork runtime-owned shared heap state for one child world.
    pub(crate) fn fork(&self, collector: Arc<Collector>) -> RuntimeResult<Self> {
        let shared = Arc::new(self.shared.fork().map_err(Box::<RuntimeError>::from)?);

        Ok(Self::from_shared(self.allocator.clone(), shared, collector))
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

    /// Borrow the lineage-owned allocator.
    pub(crate) fn allocator(&self) -> Arc<heap::Allocator> {
        self.allocator.clone()
    }

    /// Borrow the shared root set.
    pub(crate) fn roots(&self) -> &SharedRootSet {
        &self.roots
    }

    /// Borrow the shared collector work.
    pub(crate) fn collector_work(&self) -> &Arc<CollectorWork> {
        &self.collector_work
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
        self.collector_work.quiesce();
    }

    /// Resume shared GC after one quiescent operation.
    pub(crate) fn resume(&self) {
        self.collector_work.resume();

        if self.collector.mode().is_concurrent()
            && self.shared.gc_phase() != heap::SharedGcPhase::Idle
        {
            self.collector.wake(&self.collector_work);
        }
    }

    /// Wake concurrent shared GC work.
    pub(crate) fn wake(&self) {
        self.collector.wake(&self.collector_work);
    }

    /// Return whether the shared heap is currently marking.
    pub(crate) fn shared_gc_marking(&self) -> bool {
        self.shared.gc_phase() == heap::SharedGcPhase::Mark
    }

    /// Return the bounded shared local-edge scan work for one worker tick.
    pub(crate) fn shared_edge_scan_work_bytes(&self) -> usize {
        self.roots.edge_scan_work_bytes()
    }

    /// Publish discovered shared edges into the runtime root state.
    pub(crate) fn push_shared_edge_roots(
        &self,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.push_edge_roots(epoch, worker_id, roots);
        self.wake();
    }

    /// Return the active epoch when this worker owes one direct shared-root publication.
    pub(crate) fn pending_shared_root_epoch(&self, worker_id: WorkerId) -> Option<SharedRootEpoch> {
        self.roots.pending_root_epoch(worker_id)
    }

    /// Replace the direct shared roots cached for one worker.
    pub(crate) fn replace_shared_direct_roots(
        &self,
        epoch: SharedRootEpoch,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.roots.replace_direct_roots(epoch, worker_id, roots);
        self.wake();
    }

    /// Queue one worker for one later shared direct-root rescan.
    pub(crate) fn queue_shared_root_scan(&self, worker_id: WorkerId) {
        self.roots.queue_root_scan(worker_id);
    }

    /// Join one worker to an active shared mark cycle.
    pub(crate) fn join_shared_mark(&self, worker_id: WorkerId) {
        self.roots.join_mark(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    pub(crate) fn leave_shared_edge_scan(&self, worker_id: WorkerId) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.leave_edge_scan(epoch, worker_id);
        self.wake();
    }

    /// Return whether shared mark termination is waiting on worker publication.
    pub(crate) fn shared_gc_terminating(&self) -> bool {
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
    fn from_shared(
        allocator: Arc<heap::Allocator>,
        shared: Arc<heap::SharedHeap>,
        collector: Arc<Collector>,
    ) -> Self {
        let roots = Arc::new(SharedRootSet::default());
        let collector_work = CollectorWork::new(shared.clone(), roots.clone());

        Self {
            allocator,
            shared,
            roots,
            collector_work,
            collector,
        }
    }
}
