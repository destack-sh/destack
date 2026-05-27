use crate::shared::gc::SharedGcPhase;
use crate::shared::space::SharedHeapSpace;
use crate::{GcKind, GcProgress, GcStats, HeapError, HeapResult, SharedHeapReference};

/// The budget charged for one metadata-only sweep step.
const METADATA_STEP_BYTES: usize = 1;

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
        let store = self.state.read();
        self.gc
            .start_sweep(store.small.spans.len(), store.large.allocations.len());
        drop(store);

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

        // active cursor
        let mut cursor = self.gc.sweep_cursor();
        let mut swept_bytes = 0usize;
        let mut released_references = Vec::new();
        let is_complete = {
            let store = self.state.read();
            let mark_epoch = self.gc.mark_epoch();

            // small spans
            while swept_bytes < budget_bytes && cursor.small_span_index < cursor.small_span_limit {
                let span = &store.small.spans[cursor.small_span_index];
                if span.occupied_count() == 0 || span.pages_empty() {
                    swept_bytes += METADATA_STEP_BYTES;
                    cursor.small_span_index += 1;
                    cursor.small_slot_index = 0;

                    continue;
                }

                while swept_bytes < budget_bytes && cursor.small_slot_index < span.slot_count {
                    let slot_index = cursor.small_slot_index;
                    cursor.small_slot_index += 1;

                    if !span.contains_slot(slot_index) {
                        swept_bytes += METADATA_STEP_BYTES;

                        continue;
                    }

                    swept_bytes += span.class.size_class.max(1);
                    if span.is_marked(slot_index, mark_epoch) {
                        continue;
                    }

                    let slot_offset = span.class.size_class * slot_index;
                    let reference = SharedHeapReference::new(span.first_offset + slot_offset);
                    released_references.push(reference);
                }

                if cursor.small_slot_index >= span.slot_count {
                    cursor.small_span_index += 1;
                    cursor.small_slot_index = 0;
                }
            }

            // large allocations
            while swept_bytes < budget_bytes && cursor.large_index < cursor.large_limit {
                let allocation_index = cursor.large_index;
                cursor.large_index += 1;

                let allocation = store.large.allocations[allocation_index].read();
                if !allocation.is_live {
                    swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                swept_bytes += allocation.byte_len.max(1);
                if allocation.mark_epoch == mark_epoch {
                    continue;
                }

                released_references.push(SharedHeapReference::new(allocation.first_offset));
            }

            cursor.small_span_index >= cursor.small_span_limit
                && cursor.large_index >= cursor.large_limit
        };
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
        if is_complete {
            return self.finish_collection().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        let mut store = self.state.write();
        let usage = self.accounting.usage(&store, self.allocator.page_bytes());

        // cycle stats
        let (freed_allocations, freed_bytes) = self.gc.sweep_freed();
        let stats = GcStats {
            freed_allocations,
            live_allocations: usage.allocation_count,
            freed_bytes,
            allocated_bytes: usage.allocated_bytes,
            retained_bytes: usage.retained_bytes,
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
