use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::Runtime;
use tspp_heap as heap;

impl Runtime {
    /// Return whether one concurrent shared collection is still in flight.
    pub(crate) fn shared_gc_in_flight(&self) -> bool {
        self.shared_collection.is_concurrent()
            && (self.shared_heap.gc_phase() != heap::GcPhase::Idle
                || self.shared_collection.is_busy()
                || self.shared_gc_pending_cleanup())
    }

    /// Refresh shared GC work from current shared pressure.
    fn refresh_shared_gc_work(&mut self) {
        let work_bytes = self.shared_heap.take_edge_scan_work_bytes();

        self.shared_collection
            .roots()
            .set_edge_scan_work_bytes(work_bytes);
    }

    /// Start shared root publication across all workers.
    fn start_shared_root_publication(&mut self) {
        let work_bytes = self.shared_heap.take_edge_scan_work_bytes();

        self.flush_shared_caches();
        self.shared_collection
            .roots()
            .begin_mark(self.workers.keys().copied(), work_bytes);
        self.start_shared_edge_scan();
    }

    /// Finish shared root publication across all workers.
    fn finish_shared_root_publication(&mut self) {
        self.finish_shared_edge_scan();
        self.shared_collection.roots().finish_mark();
    }

    /// Remove workers that have finished the active shared local-edge pass.
    fn refresh_shared_edge_scan(&mut self) {
        let mut finished_workers = Vec::new();

        for (&worker_id, worker) in &self.workers {
            if worker.shared_edge_scan_idle() {
                finished_workers.push(worker_id);
            }
        }

        for worker_id in finished_workers {
            self.shared_collection
                .leave_edge_scan(&self.shared_heap, &self.program, worker_id);
        }
    }

    /// Run one shared GC step.
    pub(crate) fn advance_shared_gc(&mut self) -> RuntimeResult<Option<heap::GcAdvance>> {
        if self.shared_collection.is_concurrent() {
            return self.advance_shared_gc_concurrent();
        }

        self.advance_shared_gc_inline()
    }

    /// Run one shared GC step inline from the current thread.
    fn advance_shared_gc_inline(&mut self) -> RuntimeResult<Option<heap::GcAdvance>> {
        let was_active = self.shared_heap.gc_phase() != heap::GcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared_heap
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(None);
        }

        // leave Drop callbacks to a worker with executable state
        if self.shared_heap.gc_phase() == heap::GcPhase::Drop {
            return Ok(None);
        }

        if did_start {
            self.start_shared_root_publication();

            return Ok(Some(heap::GcAdvance::started(heap::GcCollector::Shared)));
        }

        self.refresh_shared_gc_work();

        // let workers publish roots before coordinator mark work resumes
        let roots_complete = if self.shared_heap.gc_phase() == heap::GcPhase::Mark {
            self.refresh_shared_edge_scan();
            self.shared_collection.roots().roots_complete()
        } else {
            true
        };
        if !roots_complete {
            return Ok(None);
        }

        let roots = self.shared_collection.roots().roots_snapshot();
        let budget_bytes = self
            .shared_heap
            .take_collection_budget_bytes(self.worker_count());
        let advance = self
            .shared_heap
            .step_collection(
                roots.as_ref(),
                roots_complete,
                budget_bytes,
                self.program.trace_view(),
            )
            .map_err(Box::<RuntimeError>::from)?;
        let is_active = self.shared_heap.gc_phase() != heap::GcPhase::Idle;

        if self.shared_heap.gc_phase() != heap::GcPhase::Mark || roots_complete {
            self.shared_collection.roots().clear_termination();
        } else if self.shared_heap.mark_idle() {
            self.shared_collection.roots().request_termination();
        }

        if !is_active && was_active {
            self.finish_shared_root_publication();
        }

        if advance.advanced() {
            Ok(Some(advance))
        } else {
            Ok(None)
        }
    }

    /// Drive shared GC through one collector thread.
    fn advance_shared_gc_concurrent(&mut self) -> RuntimeResult<Option<heap::GcAdvance>> {
        if let Some(error) = self.shared_collection.take_failure() {
            return Err(error);
        }

        if self.shared_heap.gc_phase() == heap::GcPhase::Idle && self.shared_gc_pending_cleanup() {
            self.finish_shared_root_publication();

            return Ok(None);
        }

        // leave Drop callbacks to a worker with executable state
        if self.shared_heap.gc_phase() == heap::GcPhase::Drop {
            return Ok(None);
        }

        let was_active = self.shared_heap.gc_phase() != heap::GcPhase::Idle;
        let did_start = if was_active {
            false
        } else {
            self.shared_heap
                .start_gc()
                .map_err(Box::<RuntimeError>::from)?
        };

        if !was_active && !did_start {
            return Ok(None);
        }

        if did_start {
            self.start_shared_root_publication();
            self.refresh_shared_gc_work();
            self.shared_collection
                .wake(&self.shared_heap, &self.program);

            return Ok(Some(heap::GcAdvance::started(heap::GcCollector::Shared)));
        }

        self.refresh_shared_gc_work();
        self.shared_collection
            .wake(&self.shared_heap, &self.program);

        Ok(None)
    }

    /// Return whether one completed shared cycle still needs cleanup.
    fn shared_gc_pending_cleanup(&self) -> bool {
        !self.shared_collection.roots().root_scan_idle()
            || !self.shared_collection.roots().edge_scan_idle()
            || self.shared_collection.roots().termination_requested()
    }
}
