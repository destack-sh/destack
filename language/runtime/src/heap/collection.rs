use std::sync::{Arc, Weak};

use heap::{GcPhase, SharedHeapReference, TraceView};
use parking_lot::{Condvar, Mutex};
use tspp_heap as heap;
use tspp_program as program;

use super::{WorldCollector, WorldCollectorMode};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::heap::{RootEpoch, RootSet};
use crate::worker::WorkerId;

/// Collection state for one runtime-owned shared heap.
#[derive(Debug)]
pub(crate) struct SharedCollectionState {
    /// Roots published by workers for the active mark cycle.
    roots: RootSet,
    /// World-owned collector without extending its lifetime.
    collector: Weak<WorldCollector>,
    /// World collector execution mode.
    mode: WorldCollectorMode,
    /// Collection lifecycle shared with the collector thread.
    status: Mutex<CollectionStatus>,
    /// Wake quiescence waiters when pending collection work drains.
    quiesce: Condvar,
    /// Terminal collection failure retained by the collector thread.
    failure: Mutex<Option<Box<RuntimeError>>>,
}

/// Shared collection execution status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum CollectionStatus {
    /// No collection work is queued.
    #[default]
    Idle,
    /// One collection step is queued.
    Scheduled,
    /// One collection step is running.
    Running,
    /// Collection will suspend after the running step.
    Suspending,
    /// Collection is suspended for capture or restore.
    Suspended,
}

impl SharedCollectionState {
    /// Create collection state for one shared heap.
    pub(crate) fn new(collector: &Arc<WorldCollector>) -> Arc<Self> {
        Arc::new(Self {
            roots: RootSet::default(),
            collector: Arc::downgrade(collector),
            mode: collector.mode(),
            status: Mutex::new(CollectionStatus::default()),
            quiesce: Condvar::new(),
            failure: Mutex::new(None),
        })
    }

    /// Borrow the active shared root set.
    pub(crate) const fn roots(&self) -> &RootSet {
        &self.roots
    }

    /// Return whether collector work is queued or running.
    pub(crate) fn is_busy(&self) -> bool {
        let status = *self.status.lock();

        matches!(
            status,
            CollectionStatus::Scheduled | CollectionStatus::Running | CollectionStatus::Suspending
        )
    }

    /// Return one collector failure when background collection failed.
    pub(crate) fn take_failure(&self) -> Option<Box<RuntimeError>> {
        self.failure.lock().take()
    }

    /// Suspend collection and wait for in-flight work to drain.
    pub(crate) fn quiesce(&self) {
        let mut status = self.status.lock();
        *status = match *status {
            CollectionStatus::Idle | CollectionStatus::Scheduled => CollectionStatus::Suspended,
            CollectionStatus::Running => CollectionStatus::Suspending,
            CollectionStatus::Suspending | CollectionStatus::Suspended => *status,
        };

        while *status == CollectionStatus::Suspending {
            self.quiesce.wait(&mut status);
        }
    }

    /// Resume collection after one quiescent operation.
    pub(crate) fn resume(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
    ) {
        let mut status = self.status.lock();
        if *status == CollectionStatus::Suspended {
            *status = CollectionStatus::Idle;
        }
        self.quiesce.notify_all();
        drop(status);

        if self.mode.is_concurrent() && heap.gc_phase() != GcPhase::Idle {
            self.wake(heap, program);
        }
    }

    /// Wake concurrent shared collection work.
    pub(crate) fn wake(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
    ) {
        if !self.mode.is_concurrent() {
            return;
        }

        let Some(collector) = self.collector.upgrade() else {
            self.fail("world collector is unavailable");

            return;
        };

        collector.wake(self, heap, program);
    }

    /// Return bounded local-edge scan work for one worker run.
    pub(crate) fn edge_scan_work_bytes(&self) -> usize {
        self.roots.edge_scan_work_bytes()
    }

    /// Publish discovered shared edges into the active root set.
    pub(crate) fn push_edge_roots(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.push_edge_roots(epoch, worker_id, roots);
        self.wake(heap, program);
    }

    /// Return the active epoch when this worker owes direct root publication.
    pub(crate) fn pending_root_epoch(&self, worker_id: WorkerId) -> Option<RootEpoch> {
        self.roots.pending_root_epoch(worker_id)
    }

    /// Replace direct shared roots cached for one worker.
    pub(crate) fn replace_direct_roots(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
        epoch: RootEpoch,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.roots.replace_direct_roots(epoch, worker_id, roots);
        self.wake(heap, program);
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
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
    ) {
        let Some(epoch) = self.roots.active_epoch() else {
            return;
        };

        self.roots.leave_edge_scan(epoch, worker_id);
        self.wake(heap, program);
    }

    /// Return whether mark termination awaits worker publication.
    pub(crate) fn is_terminating(&self) -> bool {
        self.roots.termination_requested()
    }

    /// Remove one worker from root publication state.
    pub(crate) fn remove_worker(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
        worker_id: WorkerId,
    ) {
        self.roots.remove_worker(worker_id);
        self.wake(heap, program);
    }

    /// Return whether concurrent collection is enabled.
    pub(crate) fn is_concurrent(&self) -> bool {
        self.mode.is_concurrent()
    }

    /// Run one scheduled collector step.
    pub(super) fn run_scheduled(
        self: &Arc<Self>,
        heap: &Arc<heap::SharedHeap>,
        program: &Arc<program::Program>,
    ) {
        if !self.begin_run() {
            return;
        }

        let should_continue = self.collect_with_failure(heap, program.trace_view());
        self.finish_run();

        if should_continue {
            self.wake(heap, program);
        }
    }

    /// Schedule one collector step.
    pub(super) fn schedule(&self) -> bool {
        let mut status = self.status.lock();
        if *status != CollectionStatus::Idle {
            return false;
        }

        *status = CollectionStatus::Scheduled;

        true
    }

    /// Clear scheduled work after one collector scheduling failure.
    pub(super) fn fail_scheduled(&self, message: impl Into<String>) {
        let mut status = self.status.lock();
        if *status == CollectionStatus::Scheduled {
            *status = CollectionStatus::Idle;
        }
        self.quiesce.notify_all();
        drop(status);

        self.fail(message);
    }

    /// Mark one scheduled collector step as running.
    fn begin_run(&self) -> bool {
        let mut status = self.status.lock();
        if *status != CollectionStatus::Scheduled {
            self.quiesce.notify_all();

            return false;
        }

        *status = CollectionStatus::Running;

        true
    }

    /// Mark the running collector step as finished.
    fn finish_run(&self) {
        let mut status = self.status.lock();
        *status = if *status == CollectionStatus::Suspending {
            CollectionStatus::Suspended
        } else {
            CollectionStatus::Idle
        };
        self.quiesce.notify_all();
    }

    /// Retain one terminal collection failure.
    fn fail(&self, message: impl Into<String>) {
        *self.failure.lock() = Some(
            RuntimeError::Internal {
                message: message.into(),
            }
            .boxed(),
        );
    }

    /// Run one bounded collection increment and retain failures.
    fn collect_with_failure(&self, heap: &heap::SharedHeap, trace_view: TraceView<'_>) -> bool {
        match self.collect(heap, trace_view) {
            Ok(should_continue) => should_continue,
            Err(error) => {
                *self.failure.lock() = Some(error);

                false
            }
        }
    }

    /// Run one bounded shared collection increment.
    fn collect(&self, heap: &heap::SharedHeap, trace_view: TraceView<'_>) -> RuntimeResult<bool> {
        if heap.gc_phase() == GcPhase::Idle {
            return Ok(false);
        }

        let roots = self.roots.roots_snapshot();
        let roots_complete = self.roots.roots_complete();
        let budget_bytes = heap.take_collection_budget_bytes(1);
        let progress = heap
            .step_collection(roots.as_ref(), roots_complete, budget_bytes, trace_view)
            .map_err(Box::<RuntimeError>::from)?;

        if heap.gc_phase() != GcPhase::Mark || roots_complete {
            self.roots.clear_termination();
        } else if heap.mark_idle() {
            self.roots.request_termination();
        }

        let is_waiting_on_roots =
            heap.gc_phase() == GcPhase::Mark && heap.mark_idle() && !roots_complete;
        let should_continue_mark = heap.gc_phase() == GcPhase::Mark && !is_waiting_on_roots;

        Ok(progress.advanced() || should_continue_mark)
    }
}
