use crate::shared::gc::SharedGcPhase;
use crate::shared::space::SharedHeapSpace;
use crate::{GcKind, GcProgress, GcStats, HeapError, HeapResult};

impl SharedHeapSpace {
    /// Transition from concurrent mark into sweeping.
    pub(crate) fn try_start_sweep(&self) -> HeapResult<bool> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != SharedGcPhase::Mark {
            return Err(HeapError::SharedCollectionNotMarking);
        }

        // termination
        self.gc.close_mark_publication();
        if !self.gc.mark_drained() {
            self.gc.open_mark_publication();

            return Ok(false);
        }

        // sweep state
        let references = self.live_references()?;
        self.gc.start_sweep(references);
        self.gc.set_phase(SharedGcPhase::Sweep);

        Ok(true)
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn sweep_step(&self, budget_bytes: usize) -> HeapResult<GcProgress> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        match self.gc.phase() {
            SharedGcPhase::Sweep => {}
            SharedGcPhase::Idle => return Ok(GcProgress::Idle),
            SharedGcPhase::Mark => return Err(HeapError::SharedCollectionNotSweeping),
        }

        // snapshot active sweep candidates
        let references = self.gc.sweep_references();
        let reference_len = references.len();
        let mut cursor = self.gc.sweep_cursor();

        let mut swept_bytes = 0usize;
        let mut released_references = Vec::new();

        // sweep cursor
        while swept_bytes < budget_bytes {
            if cursor >= reference_len {
                break;
            }

            let index = cursor;
            cursor += 1;
            let Some(reference) = references.get(index).copied() else {
                continue;
            };
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };
            swept_bytes += location.byte_len.max(1);

            // keep reachable allocations
            if self.is_marked_place(location.place)? {
                continue;
            }

            released_references.push(reference);
        }
        self.gc.set_sweep_cursor(cursor);

        // reclaimed allocation
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;
        for reference in released_references {
            let released_bytes = self.free(reference)?;

            freed_allocations += 1;
            freed_bytes += released_bytes;
        }
        if freed_allocations > 0 {
            self.gc.record_sweep_freed(freed_allocations, freed_bytes);
        }

        // end of sweep
        if cursor >= reference_len {
            return self.finish_collection().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        let mut store = self.state.write();
        let retained_bytes = self.live_retained_bytes(&store);
        let (live_allocations, allocated_bytes) = self.live_allocated_usage(&store);

        // cycle stats
        let (freed_allocations, freed_bytes) = self.gc.sweep_freed();
        let stats = GcStats {
            freed_allocations,
            live_allocations,
            freed_bytes,
            allocated_bytes,
            retained_bytes,
        };

        // cycle reset
        self.gc.close_mark_publication();
        self.gc.reset_sweep();
        self.gc.trace_queue.clear();

        // cycle summary
        store.gc.record_cycle(GcKind::Full, stats);

        // idle publication
        self.gc.set_phase(SharedGcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }
}
