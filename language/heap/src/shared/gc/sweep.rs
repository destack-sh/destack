use crate::shared::gc::{GcPhase, SweepCursor};
use crate::shared::storage::HeapStorage;
use crate::{
    GcKind, GcProgress, GcStats, HeapError, HeapGcStateError, HeapResult, SharedHeapReference,
};

/// The budget charged for one metadata-only sweep step.
const METADATA_STEP_BYTES: usize = 1;

impl HeapStorage {
    /// Transition from concurrent mark into sweeping.
    pub(crate) fn start_sweep_when_drained(&self) -> HeapResult<bool> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();

        // phase
        if self.gc.phase() != GcPhase::Mark {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotMarking));
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
            .start_sweep(store.small.spans.len(), store.large.blocks.len());
        drop(store);

        self.gc.set_phase(GcPhase::Sweep);

        Ok(true)
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn step_sweep(&self, budget_bytes: usize) -> HeapResult<GcProgress> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        match self.gc.phase() {
            GcPhase::Sweep => {}
            GcPhase::Idle => return Ok(GcProgress::Idle),
            GcPhase::Mark => {
                return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotSweeping));
            }
        }

        // active cursor
        let mut cursor = self.gc.sweep_cursor();
        let mut swept_bytes = 0usize;
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // free unmarked references as they are selected
        while swept_bytes < budget_bytes {
            let Some(reference) =
                self.select_unmarked_reference(&mut cursor, budget_bytes, &mut swept_bytes)?
            else {
                break;
            };

            self.gc.set_sweep_cursor(cursor);
            let released_bytes = self.free(reference)?;
            freed_allocations += 1;
            freed_bytes += released_bytes;
        }

        // publish the final sweep cursor
        let is_complete = cursor.small_span_index >= cursor.small_span_limit
            && cursor.large_index >= cursor.large_limit;
        self.gc.set_sweep_cursor(cursor);

        // reclaimed blocks
        if freed_allocations > 0 {
            self.gc.record_sweep_freed(freed_allocations, freed_bytes);
        }

        // end of sweep
        if is_complete {
            return self.finish_collection().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Select the next unmarked reference found by shared sweep.
    fn select_unmarked_reference(
        &self,
        cursor: &mut SweepCursor,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<Option<SharedHeapReference>> {
        // shared state remains read locked while selecting
        let store = self.state.read();
        let mark_epoch = self.gc.mark_epoch();

        // small spans
        while *swept_bytes < budget_bytes && cursor.small_span_index < cursor.small_span_limit {
            let span = &store.small.spans[cursor.small_span_index];

            // empty spans only charge metadata work
            if span.occupied_count() == 0 || span.pages_empty() {
                *swept_bytes += METADATA_STEP_BYTES;
                cursor.small_span_index += 1;
                cursor.small_slot_index = 0;

                continue;
            }

            while *swept_bytes < budget_bytes && cursor.small_slot_index < span.slot_count {
                let slot_index = cursor.small_slot_index;
                cursor.small_slot_index += 1;

                // empty slots only charge metadata work
                if !span.contains_slot(slot_index) {
                    *swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                *swept_bytes += span.class.size_class.max(1);
                if span.is_marked(slot_index, mark_epoch) {
                    continue;
                }

                // unmarked slots are dead
                let slot_offset = span.class.size_class * slot_index;
                let reference = SharedHeapReference::new(span.first_offset + slot_offset);

                return Ok(Some(reference));
            }

            // advance after the span drains
            if cursor.small_slot_index >= span.slot_count {
                cursor.small_span_index += 1;
                cursor.small_slot_index = 0;
            }
        }

        // large blocks
        while *swept_bytes < budget_bytes && cursor.large_index < cursor.large_limit {
            // select the next large block
            let block_index = cursor.large_index;
            cursor.large_index += 1;

            // inactive blocks only charge metadata work
            let block = store.large.blocks[block_index].read();
            if !block.is_live {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            }

            // marked blocks survive this cycle
            *swept_bytes += block.byte_len.max(1);
            if block.mark_epoch == mark_epoch {
                continue;
            }

            // unmarked blocks are dead
            return Ok(Some(SharedHeapReference::new(block.first_offset)));
        }

        Ok(None)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // collect final usage under the shared heap lock
        let mut store = self.state.write();
        let usage = self
            .accounting
            .usage(&store, self.allocator.page_size_bytes());

        // build cycle stats
        let freed = self.gc.sweep_freed();
        let stats = GcStats {
            freed_allocations: freed.allocations,
            live_allocations: usage.allocation_count,
            freed_bytes: freed.bytes,
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
        self.gc.set_phase(GcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }
}
