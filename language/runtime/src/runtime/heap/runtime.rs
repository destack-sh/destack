use std::sync::Arc;

use destack_heap::{self as heap, SharedHeapReference};
use destack_memory::MemoryMap;
use destack_program as program;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::heap::{SharedRootEpoch, SharedRootSet, resolve_shared_heap_options};
use crate::runtime::{SharedCollector, SharedGc, WorkerId};
use destack_repository::RuntimeOptions;

/// Runtime-owned heap and GC state.
#[derive(Debug)]
pub(crate) struct RuntimeHeap {
    /// World-owned memory backing runtime and worker heaps.
    pub(crate) memory: Arc<MemoryMap>,
    /// Shared heap visible to every worker in this runtime.
    pub(crate) shared: Arc<heap::SharedHeap>,
    /// Durable program owning executable metadata for this heap.
    program: Arc<program::Program>,
    /// Shared heap roots published by workers for the active mark cycle.
    roots: Arc<SharedRootSet>,
    /// Runtime-owned GC state for this shared heap.
    gc: Arc<SharedGc>,
    /// World-owned GC scheduler.
    collector: Arc<SharedCollector>,
}

impl RuntimeHeap {
    /// Create runtime-owned shared heap state from runtime options.
    pub(crate) fn new(
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
        options: &RuntimeOptions,
        program: Arc<program::Program>,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::new(
                memory.clone(),
                shared_heap_options.limits,
                shared_heap_options.options,
            )
            .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_heap(memory, shared, collector, program))
    }

    /// Restore runtime-owned shared heap state from a snapshot.
    pub(crate) fn from_snapshot(
        snapshot: &heap::SharedHeapSnapshot,
        options: &RuntimeOptions,
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
        program: Arc<program::Program>,
    ) -> RuntimeResult<Self> {
        let shared_heap_options = resolve_shared_heap_options(&options.heap)?;
        let shared = Arc::new(
            heap::SharedHeap::from_snapshot(snapshot, shared_heap_options.limits, memory.clone())
                .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_heap(memory, shared, collector, program))
    }

    /// Fork runtime-owned shared heap state for one child world.
    pub(crate) fn fork(
        &self,
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Self> {
        let shared = Arc::new(
            self.shared
                .fork(memory.clone())
                .map_err(Box::<RuntimeError>::from)?,
        );

        Ok(Self::from_heap(
            memory,
            shared,
            collector,
            self.program.clone(),
        ))
    }

    /// Capture the shared heap snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<heap::SharedHeapSnapshot> {
        self.shared
            .image()
            .map(|image| image.snapshot())
            .map_err(Box::<RuntimeError>::from)
    }

    /// Return the durable program backing this runtime heap.
    pub(crate) fn program(&self) -> &Arc<program::Program> {
        &self.program
    }

    /// Borrow the shared root set.
    pub(crate) fn roots(&self) -> &SharedRootSet {
        &self.roots
    }

    /// Register one shared mark worker.
    pub(crate) fn register_mark_worker(&self) -> heap::SharedMarkWorker {
        self.shared.register_mark_worker()
    }

    /// Return whether shared GC work is active.
    pub(crate) fn gc_busy(&self) -> bool {
        self.gc.is_busy()
    }

    /// Return one shared collector failure if one was recorded.
    pub(crate) fn take_gc_failure(&self) -> Option<Box<RuntimeError>> {
        self.gc.take_failure()
    }

    /// Suspend shared GC and wait for in-flight work to drain.
    pub(crate) fn quiesce(&self) {
        self.gc.quiesce();
    }

    /// Resume shared GC after one quiescent operation.
    pub(crate) fn resume(&self) {
        self.gc.resume();

        if self.collector.mode().is_concurrent() && self.shared.gc_phase() != heap::GcPhase::Idle {
            self.collector.wake(&self.gc, self.program());
        }
    }

    /// Wake concurrent shared GC work.
    pub(crate) fn wake(&self) {
        self.collector.wake(&self.gc, self.program());
    }

    /// Return whether the shared heap is currently marking.
    pub(crate) fn is_marking(&self) -> bool {
        self.shared.gc_phase() == heap::GcPhase::Mark
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

    /// Return whether concurrent GC is enabled.
    pub(crate) fn is_concurrent(&self) -> bool {
        self.collector.mode().is_concurrent()
    }

    /// Build state around one already-created shared heap.
    fn from_heap(
        memory: Arc<MemoryMap>,
        shared: Arc<heap::SharedHeap>,
        collector: Arc<SharedCollector>,
        program: Arc<program::Program>,
    ) -> Self {
        let roots = Arc::new(SharedRootSet::default());
        let gc = SharedGc::new(shared.clone(), roots.clone());

        Self {
            memory,
            shared,
            program,
            roots,
            gc,
            collector,
        }
    }
}
