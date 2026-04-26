use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::{Runtime, WorkerId};
use destack_heap::SharedGcPhase;

impl Runtime {
    /// Return whether the runtime shared heap is currently marking.
    #[inline]
    pub(crate) fn shared_gc_marking(&self) -> bool {
        self.shared.shared().gc_phase() == SharedGcPhase::Mark
    }

    /// Queue one worker for one later shared direct-root rescan.
    #[inline]
    pub(crate) fn queue_shared_root_scan(&self, worker_id: WorkerId) {
        self.shared.mark_roots().queue_root_scan(worker_id);
    }

    /// Record one worker as participating in the active shared local-edge pass.
    #[inline]
    pub(crate) fn join_shared_edge_scan(&self, worker_id: WorkerId) {
        self.shared.join_shared_edge_scan(worker_id);
    }

    /// Return whether one concurrent shared-GC cycle is still in flight.
    pub(crate) fn shared_gc_in_flight(&self) -> bool {
        self.shared.is_concurrent()
            && (self.shared.shared().gc_phase() != SharedGcPhase::Idle
                || self.shared.collection().is_busy()
                || self.shared_cycle_pending_cleanup())
    }

    /// Refresh shared GC work budgets from current shared pressure.
    fn refresh_shared_gc_budget(&mut self) {
        let edge_scan_budget = self.shared.shared().edge_scan_budget(self.worker_count());

        self.shared
            .mark_roots()
            .set_edge_scan_budget(edge_scan_budget);
    }

    /// Start one incremental shared direct-root pass across all workers.
    fn start_shared_root_scan(&mut self) {
        let worker_ids = self.worker_ids();

        self.shared.mark_roots().clear_direct_roots();
        self.shared.mark_roots().clear_root_scan();

        for worker_id in worker_ids {
            self.shared.mark_roots().queue_root_scan(worker_id);
        }
    }

    /// Start local-to-shared edge scans across all workers.
    fn start_shared_edge_scan_for_gc(&mut self) {
        let worker_ids = self.worker_ids();
        let edge_scan_budget = self.shared.shared().edge_scan_budget(self.worker_count());

        self.start_shared_edge_scan();
        self.shared.mark_roots().clear_edge_scan();
        self.shared
            .mark_roots()
            .set_edge_scan_budget(edge_scan_budget);

        for worker_id in worker_ids {
            self.shared.mark_roots().join_edge_scan(worker_id);
        }
    }

    /// Finish local-to-shared edge scans across all workers.
    fn finish_shared_edge_scan_for_gc(&mut self) {
        self.finish_shared_edge_scan();
        self.shared.mark_roots().clear_edge_scan();
    }

    /// Remove workers that have finished the active shared local-edge pass.
    fn refresh_shared_edge_scan(&mut self) {
        let mut finished_workers = Vec::new();

        for worker_id in self.worker_ids() {
            let is_done = self
                .worker(worker_id)
                .map(|worker| worker.shared_edge_scan_idle())
                .unwrap_or(false);

            if is_done {
                finished_workers.push(worker_id);
            }
        }

        for worker_id in finished_workers {
            self.shared.mark_roots().leave_edge_scan(worker_id);
        }
    }

    /// Run one shared GC step.
    pub(crate) fn tick_shared_gc(&mut self) -> RuntimeResult<bool> {
        if self.shared.is_concurrent() {
            return self.advance_shared_gc_concurrent();
        }

        self.advance_shared_gc_inline()
    }

    /// Run one shared GC step inline from the current thread.
    fn advance_shared_gc_inline(&mut self) -> RuntimeResult<bool> {
        let was_active = self.shared.shared().gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared
                .shared()
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(false);
        }

        if did_start {
            self.shared.mark_roots().clear_termination();
            self.shared.mark_roots().clear_edge_scan();
            self.start_shared_root_scan();
            self.start_shared_edge_scan_for_gc();
        }

        self.refresh_shared_gc_budget();

        let before_cycles = self.shared.shared().gc_state().completed_cycles;
        let mut roots_complete = true;

        if did_start || self.shared.shared().gc_phase() == SharedGcPhase::Mark {
            self.refresh_shared_edge_scan();
            roots_complete = self.shared.mark_roots().roots_complete();
        }

        let roots = self.shared.mark_roots().roots_snapshot();
        let work_items = self
            .shared
            .shared()
            .take_collection_budget(self.worker_count());
        let progress = self
            .shared
            .shared()
            .gc_step(&roots, roots_complete, work_items)
            .map_err(Box::<RuntimeError>::from)?;
        let is_active = self.shared.shared().gc_phase() != SharedGcPhase::Idle;
        let after_cycles = self.shared.shared().gc_state().completed_cycles;

        if self.shared.shared().gc_phase() != SharedGcPhase::Mark || roots_complete {
            self.shared.mark_roots().clear_termination();
        } else if self.shared.shared().mark_idle() {
            self.shared.mark_roots().request_termination();
        }

        if !is_active && (was_active || did_start) {
            self.shared.mark_roots().clear_termination();
            self.shared.mark_roots().clear_root_scan();
            self.finish_shared_edge_scan_for_gc();
        }

        Ok(progress.made_progress() || was_active || is_active || before_cycles != after_cycles)
    }

    /// Drive shared collection through one collector thread.
    fn advance_shared_gc_concurrent(&mut self) -> RuntimeResult<bool> {
        if let Some(error) = self.shared.collection().take_failure() {
            return Err(error);
        }

        if self.shared.shared().gc_phase() == SharedGcPhase::Idle
            && self.shared_cycle_pending_cleanup()
        {
            self.shared.mark_roots().clear_termination();
            self.shared.mark_roots().clear_root_scan();
            self.finish_shared_edge_scan_for_gc();

            return Ok(true);
        }

        let was_active = self.shared.shared().gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared
                .shared()
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(false);
        }

        if did_start {
            self.shared.mark_roots().clear_termination();
            self.shared.mark_roots().clear_edge_scan();
            self.start_shared_root_scan();
            self.start_shared_edge_scan_for_gc();
        }

        self.refresh_shared_gc_budget();
        self.shared.wake();

        Ok(did_start)
    }

    /// Return whether one completed shared cycle still needs cleanup.
    fn shared_cycle_pending_cleanup(&self) -> bool {
        !self.shared.mark_roots().root_scan_idle()
            || !self.shared.mark_roots().edge_scan_idle()
            || self.shared.mark_roots().termination_requested()
    }
}
