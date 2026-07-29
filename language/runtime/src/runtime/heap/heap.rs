use std::sync::Arc;

use destack_heap as heap;
use destack_memory::MemoryMap;
use destack_program as program;
use destack_repository::RuntimeOptions;
use heap::{GcPhase, SharedHeapReference, TraceView};
use parking_lot::{Condvar, Mutex};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::SharedCollector;
use crate::runtime::heap::{
    RootEpoch, RootSet, resolve_local_heap_options, resolve_shared_heap_options,
};
use crate::worker::WorkerId;

/// Runtime-owned shared heap and collection state.
#[derive(Debug)]
pub(crate) struct SharedHeap {
    /// Physical shared heap visible to every worker.
    pub(crate) shared: heap::SharedHeap,
    /// Allocation plans indexed by Program allocation site id.
    allocation_plans: Arc<[Option<heap::AllocationPlan>]>,
    /// Roots published by workers for the active mark cycle.
    roots: RootSet,
    /// World-owned collector scheduler.
    collector: Arc<SharedCollector>,
    /// Collection state shared with the collector thread.
    state: Mutex<CollectionState>,
    /// Wake quiescence waiters when pending collection work drains.
    quiesce: Condvar,
    /// Terminal collection failure retained by the collector thread.
    failure: Mutex<Option<Box<RuntimeError>>>,
}

/// Shared collection lifecycle state.
#[derive(Debug, Default)]
struct CollectionState {
    /// Pending collector work.
    pending: CollectionPending,
    /// Whether the collector thread is running one step.
    is_running: bool,
}

/// Pending shared collection work.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CollectionPending {
    /// No collector work is queued.
    #[default]
    Idle,
    /// One collector wake is queued.
    Scheduled,
    /// Collection is suspended for capture or restore.
    Suspended,
}

impl SharedHeap {
    /// Create runtime-owned shared heap state from runtime options.
    pub(crate) fn new(
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
        options: &RuntimeOptions,
        program: &program::Program,
    ) -> RuntimeResult<Arc<Self>> {
        let local_options = resolve_local_heap_options(&options.heap)?;
        let shared_options = resolve_shared_heap_options(&options.heap)?;
        let shared = heap::SharedHeap::new(memory, shared_options.limits, shared_options.options)
            .map_err(Box::<RuntimeError>::from)?;
        let allocation_plans = program
            .plan_allocations(&local_options.options, shared.options())?
            .into();

        Ok(Self::from_heap(shared, collector, allocation_plans))
    }

    /// Restore runtime-owned shared heap state from an image.
    pub(crate) fn from_image(
        image: &heap::SharedHeapImage,
        options: &RuntimeOptions,
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
        program: &program::Program,
    ) -> RuntimeResult<Arc<Self>> {
        let local_options = resolve_local_heap_options(&options.heap)?;
        let shared_options = resolve_shared_heap_options(&options.heap)?;
        let shared = heap::SharedHeap::from_image_with_limits(image, memory, shared_options.limits)
            .map_err(Box::<RuntimeError>::from)?;
        let allocation_plans = program
            .plan_allocations(&local_options.options, shared.options())?
            .into();

        Ok(Self::from_heap(shared, collector, allocation_plans))
    }

    /// Fork runtime-owned shared heap state for one child world.
    pub(crate) fn fork(
        &self,
        memory: Arc<MemoryMap>,
        collector: Arc<SharedCollector>,
    ) -> RuntimeResult<Arc<Self>> {
        let shared = self
            .shared
            .fork(memory)
            .map_err(Box::<RuntimeError>::from)?;

        Ok(Self::from_heap(
            shared,
            collector,
            self.allocation_plans.clone(),
        ))
    }

    /// Capture the physical shared heap.
    pub(crate) fn image(&self) -> heap::SharedHeapImage {
        self.shared.image()
    }

    /// Return the memory map backing runtime and worker heaps.
    pub(crate) fn memory(&self) -> &Arc<MemoryMap> {
        self.shared.memory()
    }

    /// Return allocation plans indexed by Program allocation site id.
    pub(crate) fn allocation_plans(&self) -> &[Option<heap::AllocationPlan>] {
        &self.allocation_plans
    }

    /// Borrow the active shared root set.
    pub(crate) const fn roots(&self) -> &RootSet {
        &self.roots
    }

    /// Register one shared mark worker.
    pub(crate) fn register_mark_worker(&self) -> heap::SharedMarkWorker {
        self.shared.register_mark_worker()
    }

    /// Return whether collector work is queued or running.
    pub(crate) fn is_busy(&self) -> bool {
        let state = self.state.lock();

        state.is_running || state.pending == CollectionPending::Scheduled
    }

    /// Return one collector failure when background collection failed.
    pub(crate) fn take_failure(&self) -> Option<Box<RuntimeError>> {
        self.failure.lock().take()
    }

    /// Suspend collection and wait for in-flight work to drain.
    pub(crate) fn quiesce(&self) {
        let mut state = self.state.lock();
        state.pending = CollectionPending::Suspended;

        while state.is_running || state.pending == CollectionPending::Scheduled {
            self.quiesce.wait(&mut state);
        }
    }

    /// Resume collection after one quiescent operation.
    pub(crate) fn resume(self: &Arc<Self>, program: &Arc<program::Program>) {
        let mut state = self.state.lock();
        if state.pending == CollectionPending::Suspended {
            state.pending = CollectionPending::Idle;
        }
        self.quiesce.notify_all();
        drop(state);

        if self.collector.mode().is_concurrent() && self.shared.gc_phase() != GcPhase::Idle {
            self.wake(program);
        }
    }

    /// Wake concurrent shared collection work.
    pub(crate) fn wake(self: &Arc<Self>, program: &Arc<program::Program>) {
        self.collector.wake(self, program);
    }

    /// Return whether the shared heap is currently marking.
    pub(crate) fn is_marking(&self) -> bool {
        self.shared.gc_phase() == GcPhase::Mark
    }

    /// Return bounded local-edge scan work for one worker run.
    pub(crate) fn edge_scan_work_bytes(&self) -> usize {
        self.roots.edge_scan_work_bytes()
    }

    /// Publish discovered shared edges into the active root set.
    pub(crate) fn push_edge_roots(
        self: &Arc<Self>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.push_edge_roots(epoch, worker_id, roots);
        self.wake(program);
    }

    /// Return the active epoch when this worker owes direct root publication.
    pub(crate) fn pending_root_epoch(&self, worker_id: WorkerId) -> Option<RootEpoch> {
        self.roots.pending_root_epoch(worker_id)
    }

    /// Replace direct shared roots cached for one worker.
    pub(crate) fn replace_direct_roots(
        self: &Arc<Self>,
        program: &Arc<program::Program>,
        epoch: RootEpoch,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.roots.replace_direct_roots(epoch, worker_id, roots);
        self.wake(program);
    }

    /// Queue one worker for a later direct root rescan.
    pub(crate) fn queue_root_scan(&self, worker_id: WorkerId) {
        self.roots.queue_root_scan(worker_id);
    }

    /// Join one worker to the active mark cycle.
    pub(crate) fn join_mark(&self, worker_id: WorkerId) {
        self.roots.join_mark(worker_id);
    }

    /// Remove one worker from the active local-edge pass.
    pub(crate) fn leave_edge_scan(
        self: &Arc<Self>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
    ) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.leave_edge_scan(epoch, worker_id);
        self.wake(program);
    }

    /// Return whether mark termination awaits worker publication.
    pub(crate) fn is_terminating(&self) -> bool {
        self.roots.termination_requested()
    }

    /// Remove one worker from root publication state.
    pub(crate) fn remove_worker(
        self: &Arc<Self>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
    ) {
        self.roots.remove_worker(worker_id);
        self.wake(program);
    }

    /// Return whether concurrent collection is enabled.
    pub(crate) fn is_concurrent(&self) -> bool {
        self.collector.mode().is_concurrent()
    }

    /// Run one scheduled collector step.
    pub(super) fn run_scheduled(
        self: &Arc<Self>,
        collector: &Arc<SharedCollector>,
        program: &Arc<program::Program>,
    ) {
        if !self.begin_run() {
            return;
        }

        let should_continue = self.collect_with_failure(program.trace_view());
        self.finish_run();

        if should_continue {
            collector.wake(self, program);
        }
    }

    /// Schedule one collector step.
    pub(super) fn schedule(&self) -> bool {
        let mut state = self.state.lock();
        if state.pending != CollectionPending::Idle {
            return false;
        }

        state.pending = CollectionPending::Scheduled;

        true
    }

    /// Clear scheduled work after one collector scheduling failure.
    pub(super) fn fail_scheduled(&self, message: impl Into<String>) {
        let mut state = self.state.lock();
        if state.pending == CollectionPending::Scheduled {
            state.pending = CollectionPending::Idle;
        }
        self.quiesce.notify_all();
        drop(state);

        *self.failure.lock() = Some(
            RuntimeError::Internal {
                message: message.into(),
            }
            .boxed(),
        );
    }

    /// Mark one scheduled collector step as running.
    fn begin_run(&self) -> bool {
        let mut state = self.state.lock();
        if state.pending != CollectionPending::Scheduled {
            self.quiesce.notify_all();

            return false;
        }

        state.pending = CollectionPending::Idle;
        state.is_running = true;

        true
    }

    /// Mark the running collector step as finished.
    fn finish_run(&self) {
        let mut state = self.state.lock();
        state.is_running = false;
        self.quiesce.notify_all();
    }

    /// Run one bounded collection increment and retain failures.
    fn collect_with_failure(&self, trace_view: TraceView<'_>) -> bool {
        match self.collect(trace_view) {
            Ok(should_continue) => should_continue,
            Err(error) => {
                *self.failure.lock() = Some(error);

                false
            }
        }
    }

    /// Run one bounded shared collection increment.
    fn collect(&self, trace_view: TraceView<'_>) -> RuntimeResult<bool> {
        if self.shared.gc_phase() == GcPhase::Idle {
            return Ok(false);
        }

        let roots = self.roots.roots_snapshot();
        let roots_complete = self.roots.roots_complete();
        let budget_bytes = self.shared.take_collection_budget_bytes(1);
        let progress = self
            .shared
            .step_collection(roots.as_ref(), roots_complete, budget_bytes, trace_view)
            .map_err(Box::<RuntimeError>::from)?;

        if self.shared.gc_phase() != GcPhase::Mark || roots_complete {
            self.roots.clear_termination();
        } else if self.shared.mark_idle() {
            self.roots.request_termination();
        }

        let is_waiting_on_roots =
            self.shared.gc_phase() == GcPhase::Mark && self.shared.mark_idle() && !roots_complete;
        let should_continue_mark = self.shared.gc_phase() == GcPhase::Mark && !is_waiting_on_roots;

        Ok(progress.advanced() || should_continue_mark)
    }

    /// Build runtime state around one physical shared heap.
    fn from_heap(
        shared: heap::SharedHeap,
        collector: Arc<SharedCollector>,
        allocation_plans: Arc<[Option<heap::AllocationPlan>]>,
    ) -> Arc<Self> {
        Arc::new(Self {
            shared,
            allocation_plans,
            roots: RootSet::default(),
            collector,
            state: Mutex::new(CollectionState::default()),
            quiesce: Condvar::new(),
            failure: Mutex::new(None),
        })
    }
}
