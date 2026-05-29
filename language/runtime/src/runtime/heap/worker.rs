use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::Runtime;
use destack_heap::SharedGcPhase;

impl Runtime {
    /// Return whether one concurrent shared-GC cycle is still in flight.
    pub(crate) fn shared_gc_in_flight(&self) -> bool {
        self.heap.is_concurrent()
            && (self.heap.shared.gc_phase() != SharedGcPhase::Idle
                || self.heap.gc_busy()
                || self.shared_gc_pending_cleanup())
    }

    /// Refresh shared GC work from current shared pressure.
    fn refresh_shared_gc_work(&mut self) {
        let work_bytes = self.heap.shared.take_edge_scan_work_bytes();

        self.heap.roots().set_edge_scan_work_bytes(work_bytes);
    }

    /// Start shared root publication across all workers.
    fn start_shared_root_publication(&mut self) {
        let worker_ids = self.worker_ids();
        let work_bytes = self.heap.shared.take_edge_scan_work_bytes();

        self.flush_shared_caches();
        self.heap.roots().begin_mark(worker_ids, work_bytes);
        self.start_shared_edge_scan();
    }

    /// Finish shared root publication across all workers.
    fn finish_shared_root_publication(&mut self) {
        self.finish_shared_edge_scan();
        self.heap.roots().finish_mark();
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
            self.heap.leave_edge_scan(worker_id);
        }
    }

    /// Run one shared GC step.
    pub(crate) fn tick_shared_gc(&mut self) -> RuntimeResult<bool> {
        if self.heap.is_concurrent() {
            return self.advance_shared_gc_concurrent();
        }

        self.advance_shared_gc_inline()
    }

    /// Run one shared GC step inline from the current thread.
    fn advance_shared_gc_inline(&mut self) -> RuntimeResult<bool> {
        let was_active = self.heap.shared.gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.heap
                .shared
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(false);
        }

        if did_start {
            self.start_shared_root_publication();
        }

        self.refresh_shared_gc_work();

        let before_cycles = self.heap.shared.gc_state().completed_cycles;
        let mut roots_complete = true;

        if did_start || self.heap.shared.gc_phase() == SharedGcPhase::Mark {
            self.refresh_shared_edge_scan();
            roots_complete = self.heap.roots().roots_complete();
        }

        let roots = self.heap.roots().roots_snapshot();
        let budget_bytes = self
            .heap
            .shared
            .take_collection_budget_bytes(self.worker_count());
        let progress = self
            .heap
            .shared
            .collect_step(
                roots.as_ref(),
                roots_complete,
                budget_bytes,
                self.heap.trace_table(),
            )
            .map_err(Box::<RuntimeError>::from)?;
        let is_active = self.heap.shared.gc_phase() != SharedGcPhase::Idle;
        let after_cycles = self.heap.shared.gc_state().completed_cycles;

        if self.heap.shared.gc_phase() != SharedGcPhase::Mark || roots_complete {
            self.heap.roots().clear_termination();
        } else if self.heap.shared.mark_idle() {
            self.heap.roots().request_termination();
        }

        if !is_active && (was_active || did_start) {
            self.finish_shared_root_publication();
        }

        Ok(progress.made_progress() || was_active || is_active || before_cycles != after_cycles)
    }

    /// Drive shared GC through one collector thread.
    fn advance_shared_gc_concurrent(&mut self) -> RuntimeResult<bool> {
        if let Some(error) = self.heap.take_gc_failure() {
            return Err(error);
        }

        if self.heap.shared.gc_phase() == SharedGcPhase::Idle && self.shared_gc_pending_cleanup() {
            self.finish_shared_root_publication();

            return Ok(true);
        }

        let was_active = self.heap.shared.gc_phase() != SharedGcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.heap
                .shared
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(false);
        }

        if did_start {
            self.start_shared_root_publication();
        }

        self.refresh_shared_gc_work();
        self.heap.wake();

        Ok(did_start)
    }

    /// Return whether one completed shared cycle still needs cleanup.
    fn shared_gc_pending_cleanup(&self) -> bool {
        !self.heap.roots().root_scan_idle()
            || !self.heap.roots().edge_scan_idle()
            || self.heap.roots().termination_requested()
    }
}
