use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::WorkerId;
use crate::runtime::memory::MarkRootSet;
use destack_heap::{SharedGcPhase, SharedHeapReference};

use super::{World, WorldRef};

impl WorldRef {
    /// Return whether the shared heap is currently in one mark phase.
    #[inline]
    pub(crate) fn shared_gc_marking(&self) -> bool {
        self.shared().gc_phase() == SharedGcPhase::Mark
    }

    /// Return the bounded shared local-edge scan budget for one worker tick.
    #[inline]
    pub(crate) fn shared_edge_scan_tick_budget(&self) -> usize {
        self.mark_roots().edge_scan_budget()
    }

    /// Borrow the current shared mark roots.
    #[inline]
    pub(crate) fn mark_roots(&self) -> &MarkRootSet {
        // safety: the execution scope owns the live mark-root pointer
        unsafe { &*self.mark_roots }
    }

    /// Publish discovered shared edges into the world root state.
    #[inline]
    pub(crate) fn push_shared_edge_roots(
        &self,
        worker_id: WorkerId,
        roots: &[SharedHeapReference],
    ) {
        self.mark_roots().push_edge_roots(worker_id, roots);
        self.wake_shared_gc();
    }

    /// Return whether this worker still owes one direct shared-root publication.
    #[inline]
    pub(crate) fn is_shared_root_scan_pending(&self, worker_id: WorkerId) -> bool {
        self.mark_roots().is_root_scan_pending(worker_id)
    }

    /// Replace the direct shared roots cached for one worker.
    #[inline]
    pub(crate) fn replace_shared_direct_roots(
        &self,
        worker_id: WorkerId,
        roots: Vec<SharedHeapReference>,
    ) {
        self.mark_roots().replace_direct_roots(worker_id, roots);
        self.wake_shared_gc();
    }

    /// Queue one worker for one later shared direct-root rescan.
    #[inline]
    pub(crate) fn queue_shared_root_scan(&self, worker_id: WorkerId) {
        self.mark_roots().queue_root_scan(worker_id);
    }

    /// Remove one worker from the active shared local-edge pass.
    #[inline]
    pub(crate) fn leave_shared_edge_scan(&self, worker_id: WorkerId) {
        self.mark_roots().leave_edge_scan(worker_id);
        self.wake_shared_gc();
    }

    /// Record one worker as participating in the active shared local-edge pass.
    #[inline]
    pub(crate) fn join_shared_edge_scan(&self, worker_id: WorkerId) {
        self.mark_roots().join_edge_scan(worker_id);
    }

    /// Return whether shared mark termination is waiting on worker publication.
    #[inline]
    pub(crate) fn shared_gc_terminating(&self) -> bool {
        self.mark_roots().termination_requested()
    }
}

impl World {
    /// Return whether shared collection runs on one collector thread.
    fn shared_gc_concurrent(&self) -> bool {
        self.collector.mode().is_concurrent()
    }

    /// Return whether one concurrent shared-GC cycle is still in flight.
    pub(crate) fn shared_gc_in_flight(&self) -> bool {
        self.shared_gc_concurrent()
            && (self.shared.gc_phase() != SharedGcPhase::Idle
                || self.collection.is_busy()
                || self.shared_cycle_pending_cleanup())
    }

    /// Return the current number of live workers.
    fn live_worker_count(&self) -> usize {
        self.runtimes
            .values()
            .map(|runtime| runtime.worker_count())
            .sum()
    }

    /// Refresh shared GC work budgets from current shared pressure.
    fn refresh_shared_gc_budget(&mut self) {
        let worker_count = self.live_worker_count();
        let edge_scan_budget = self.shared.edge_scan_budget(worker_count);

        self.mark_roots.set_edge_scan_budget(edge_scan_budget);
    }

    /// Start one incremental shared direct-root pass across all workers.
    fn start_shared_root_scan(&mut self) {
        let mut worker_ids = Vec::new();

        for runtime in self.runtimes.values() {
            worker_ids.extend(runtime.worker_ids());
        }

        self.mark_roots.clear_direct_roots();
        self.mark_roots.clear_root_scan();

        for worker_id in worker_ids {
            self.mark_roots.queue_root_scan(worker_id);
        }
    }

    /// Start local-to-shared edge scans across all runtimes.
    fn start_shared_edge_scan(&mut self) {
        let mut joined_workers = Vec::new();
        let worker_count = self.live_worker_count();
        let edge_scan_budget = self.shared.edge_scan_budget(worker_count);

        for runtime in self.runtimes.values_mut() {
            runtime.start_shared_edge_scan();

            joined_workers.extend(runtime.worker_ids());
        }

        self.mark_roots.clear_edge_scan();
        self.mark_roots.set_edge_scan_budget(edge_scan_budget);

        for worker_id in joined_workers {
            self.mark_roots.join_edge_scan(worker_id);
        }
    }

    /// Finish local-to-shared edge scans across all runtimes.
    fn finish_shared_edge_scan(&mut self) {
        for runtime in self.runtimes.values_mut() {
            runtime.finish_shared_edge_scan();
        }

        self.mark_roots.clear_edge_scan();
    }

    /// Remove workers that have finished the active shared local-edge pass.
    fn refresh_shared_edge_scan(&mut self) {
        let mut finished_workers = Vec::new();

        for runtime in self.runtimes.values() {
            for worker_id in runtime.worker_ids() {
                let is_done = runtime
                    .worker(worker_id)
                    .map(|worker| worker.shared_edge_scan_idle())
                    .unwrap_or(false);

                if is_done {
                    finished_workers.push(worker_id);
                }
            }
        }

        if !finished_workers.is_empty() {
            for worker_id in finished_workers {
                self.mark_roots.leave_edge_scan(worker_id);
            }
        }
    }

    /// Run one shared GC step from the aggregated world root set.
    pub(crate) fn advance_shared_gc(&mut self) -> RuntimeResult<bool> {
        if self.shared_gc_concurrent() {
            return self.advance_shared_gc_concurrent();
        }

        self.advance_shared_gc_inline()
    }

    /// Run one shared GC step inline from the current world thread.
    fn advance_shared_gc_inline(&mut self) -> RuntimeResult<bool> {
        let was_active = self.shared.gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared.start_gc().map_err(Box::<RuntimeError>::from)?
        };

        // idle with no pending shared work
        if !was_active && !did_start {
            return Ok(false);
        }

        // new shared cycle: enable local-to-shared edge tracking
        if did_start {
            self.mark_roots.clear_termination();
            self.mark_roots.clear_edge_scan();
            self.start_shared_root_scan();
            self.start_shared_edge_scan();
        }

        self.refresh_shared_gc_budget();

        let before_cycles = self.shared.gc_state().completed_cycles;
        let mut roots_complete = true;

        // keep feeding direct and local-to-shared edges while marking
        if did_start || self.shared.gc_phase() == SharedGcPhase::Mark {
            self.refresh_shared_edge_scan();
            roots_complete = self.mark_roots.roots_complete();
        }

        // root snapshot
        let roots = self.mark_roots.roots_snapshot();

        let work_items = self.shared.take_collection_budget(self.live_worker_count());
        let stats = self
            .shared
            .gc_step(&roots, roots_complete, work_items)
            .map_err(Box::<RuntimeError>::from)?;
        let is_active = self.shared.gc_phase() != SharedGcPhase::Idle;
        let after_cycles = self.shared.gc_state().completed_cycles;

        // termination pressure
        if self.shared.gc_phase() != SharedGcPhase::Mark || roots_complete {
            self.mark_roots.clear_termination();
        } else if self.shared.mark_idle() {
            self.mark_roots.request_termination();
        }

        // drained shared cycle: stop local-to-shared edge tracking
        if !is_active && (was_active || did_start) {
            self.mark_roots.clear_termination();
            self.mark_roots.clear_root_scan();
            self.finish_shared_edge_scan();
        }

        Ok(stats.is_some() || was_active || is_active || before_cycles != after_cycles)
    }

    /// Drive shared collection through one collector thread.
    fn advance_shared_gc_concurrent(&mut self) -> RuntimeResult<bool> {
        if let Some(error) = self.collection.take_failure() {
            return Err(error);
        }

        // cleanup
        if self.shared.gc_phase() == SharedGcPhase::Idle && self.shared_cycle_pending_cleanup() {
            self.mark_roots.clear_termination();
            self.mark_roots.clear_root_scan();
            self.finish_shared_edge_scan();

            return Ok(true);
        }

        let was_active = self.shared.gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared.start_gc().map_err(Box::<RuntimeError>::from)?
        };

        // idle with no pending shared work
        if !was_active && !did_start {
            return Ok(false);
        }

        // new cycle
        if did_start {
            self.mark_roots.clear_termination();
            self.mark_roots.clear_edge_scan();
            self.start_shared_root_scan();
            self.start_shared_edge_scan();
        }

        // current budgets
        self.refresh_shared_gc_budget();

        // collector drive
        self.collector.wake(&self.collection);

        Ok(did_start)
    }

    /// Return whether one completed shared cycle still needs world cleanup.
    fn shared_cycle_pending_cleanup(&self) -> bool {
        !self.mark_roots.root_scan_idle()
            || !self.mark_roots.edge_scan_idle()
            || self.mark_roots.termination_requested()
    }

    /// Run one shared GC tick.
    pub(crate) fn tick_shared_gc(&mut self) -> RuntimeResult<bool> {
        self.advance_shared_gc()
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::WorkerId;
    use crate::runtime::memory::MarkRootSet;
    use destack_heap::SharedHeapReference;

    /// Rebuild roots should replace stale per-worker roots and deduplicate the combined set.
    #[test]
    fn test_rebuild_roots_replaces_worker_roots() {
        let shared_gc = MarkRootSet::default();
        let first = SharedHeapReference::new(1);
        let second = SharedHeapReference::new(2);
        let third = SharedHeapReference::new(3);

        // first snapshot
        shared_gc.replace_direct_roots(WorkerId(1), vec![first, second]);
        shared_gc.push_edge_roots(WorkerId(1), &[second]);

        assert_eq!(shared_gc.roots_snapshot().as_ref(), &[first, second]);

        // worker replacement
        shared_gc.replace_direct_roots(WorkerId(1), vec![third]);

        assert_eq!(shared_gc.roots_snapshot().as_ref(), &[second, third]);
    }
}
