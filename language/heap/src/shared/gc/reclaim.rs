use crate::shared::gc::ReclaimCursor;
use crate::shared::storage::HeapStorage;
use crate::{
    DropCursor, DropPlan, DropReference, GcAdvance, GcCollector, GcPhase, GcStats, HeapError,
    HeapGcStateError, HeapResult, SharedHeapReference,
};

/// The budget charged for one metadata-only reclamation step.
const METADATA_STEP_BYTES: usize = 1;

/// One unreachable shared allocation selected for reclamation.
#[derive(Debug, Clone, Copy)]
struct UnreachableAllocation {
    /// Allocation base reference.
    reference: SharedHeapReference,
    /// Allocation byte length.
    byte_len: usize,
    /// Drop work required before reclamation.
    drop: Option<DropPlan>,
}

impl HeapStorage {
    /// Transition from concurrent mark into Drop.
    pub(crate) fn start_drop_when_drained(&self) -> HeapResult<bool> {
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

        // stable allocation-table limits
        let store = self.state.read();
        self.gc
            .start_reclaim(store.small.spans.len(), store.large.blocks.len());
        drop(store);

        self.gc.set_phase(GcPhase::Drop);

        Ok(true)
    }

    /// Complete the currently claimed shared value.
    pub(crate) fn complete_drop(&self, reference: DropReference) -> HeapResult<()> {
        let _lifecycle = self.gc.lock_lifecycle();

        // require active shared Drop
        if self.gc.phase() != GcPhase::Drop {
            return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotDropping));
        }

        self.gc.complete_drop(reference)
    }

    /// Run Drop for unreachable shared values within one byte budget.
    pub(crate) fn step_drop(
        &self,
        budget_bytes: usize,
        can_run_drop: bool,
    ) -> HeapResult<GcAdvance> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        match self.gc.phase() {
            GcPhase::Drop => {}
            GcPhase::Idle => return Ok(GcAdvance::Idle),
            GcPhase::Mark | GcPhase::PublishRoots | GcPhase::ScanEdges | GcPhase::Sweep => {
                return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotDropping));
            }
        }

        // one runtime worker must complete the current value before Drop continues
        if self.gc.is_drop_claimed() {
            return Ok(GcAdvance::Idle);
        }

        // retire one completed allocation cursor without reclaiming storage
        self.gc.retire_completed_drop();

        // continue one repeated allocation before scanning onward
        if self.gc.has_pending_drop() {
            if !can_run_drop {
                return Ok(GcAdvance::Idle);
            }

            if let Some(drop) = self.gc.continue_drop(budget_bytes)? {
                return Ok(GcAdvance::Drop(drop));
            }
        }

        let mut cursor = self.gc.reclaim_cursor();
        let mut work_bytes = 0usize;

        // run Drop for selected allocations without reclaiming storage
        while work_bytes < budget_bytes {
            let allocation_cursor = cursor;
            let Some(allocation) = self.select_unmarked_allocation(
                GcPhase::Drop,
                &mut cursor,
                budget_bytes,
                &mut work_bytes,
            )?
            else {
                break;
            };

            // suspend traversal while the runtime drops this allocation
            if let Some(drop) = allocation.drop {
                if !can_run_drop {
                    self.gc.set_reclaim_cursor(allocation_cursor);

                    return Ok(GcAdvance::Idle);
                }

                let drop_cursor = DropCursor::new(
                    GcCollector::Shared,
                    DropReference::Shared(allocation.reference),
                    allocation.byte_len,
                    drop,
                    work_bytes,
                )?;
                let drop = self.gc.claim_drop(cursor, drop_cursor, budget_bytes)?;

                return Ok(GcAdvance::Drop(drop));
            }

            // publish allocations that require no runtime callback
            self.gc.set_reclaim_cursor(cursor);
        }

        // publish the final Drop cursor
        let is_complete = cursor.small_span_index >= cursor.small_span_limit
            && cursor.large_index >= cursor.large_limit;
        self.gc.set_reclaim_cursor(cursor);

        // begin sweep only after every unreachable value completed Drop
        if is_complete {
            self.gc.restart_reclaim();
            self.gc.set_phase(GcPhase::Sweep);
        }

        Ok(GcAdvance::stepped(
            GcCollector::Shared,
            GcPhase::Drop,
            budget_bytes,
            work_bytes,
        ))
    }

    /// Perform bounded shared sweep work.
    pub(crate) fn step_sweep(&self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        // lifecycle
        let _lifecycle = self.gc.lock_lifecycle();
        match self.gc.phase() {
            GcPhase::Sweep => {}
            GcPhase::Idle => return Ok(GcAdvance::Idle),
            GcPhase::Mark | GcPhase::PublishRoots | GcPhase::ScanEdges | GcPhase::Drop => {
                return Err(HeapError::gc_state(HeapGcStateError::SharedGcNotSweeping));
            }
        }

        let mut cursor = self.gc.reclaim_cursor();
        let mut swept_bytes = 0usize;
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // reclaim unmarked allocations selected by the stable post-mark cursor
        while swept_bytes < budget_bytes {
            let Some(allocation) = self.select_unmarked_allocation(
                GcPhase::Sweep,
                &mut cursor,
                budget_bytes,
                &mut swept_bytes,
            )?
            else {
                break;
            };

            self.gc.set_reclaim_cursor(cursor);
            let released_bytes = self.free(allocation.reference)?;
            freed_allocations += 1;
            freed_bytes += released_bytes;
        }

        // publish the final sweep cursor and accounting
        let is_complete = cursor.small_span_index >= cursor.small_span_limit
            && cursor.large_index >= cursor.large_limit;
        self.gc.set_reclaim_cursor(cursor);
        if freed_allocations > 0 {
            self.gc.record_reclaimed(freed_allocations, freed_bytes);
        }

        // finish the collection after every allocation table drains
        if is_complete {
            return self.finish_collection().map(|stats| {
                GcAdvance::completed(
                    GcCollector::Shared,
                    GcPhase::Sweep,
                    budget_bytes,
                    swept_bytes,
                    stats,
                )
            });
        }

        Ok(GcAdvance::stepped(
            GcCollector::Shared,
            GcPhase::Sweep,
            budget_bytes,
            swept_bytes,
        ))
    }

    /// Select the next unreachable allocation for one post-mark phase.
    fn select_unmarked_allocation(
        &self,
        phase: GcPhase,
        cursor: &mut ReclaimCursor,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<Option<UnreachableAllocation>> {
        // shared state remains read locked while selecting
        let store = self.state.read();
        let mark_epoch = self.gc.mark_epoch();

        // small spans
        while *swept_bytes < budget_bytes && cursor.small_span_index < cursor.small_span_limit {
            let span = &store.small.spans[cursor.small_span_index];

            // skip classes that cannot require a runtime callback during Drop
            if phase == GcPhase::Drop && span.class.drop_plan().is_none() {
                *swept_bytes += METADATA_STEP_BYTES;
                cursor.small_span_index += 1;
                cursor.small_slot_index = 0;

                continue;
            }

            // empty spans only charge metadata work
            if span.occupied_count() == 0 {
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
                if span.is_marked(slot_index, mark_epoch) {
                    *swept_bytes += if phase == GcPhase::Drop {
                        METADATA_STEP_BYTES
                    } else {
                        span.class.size_class().max(1)
                    };

                    continue;
                }

                // unmarked slots are dead
                let slot_offset = span.class.size_class() * slot_index;
                let reference = SharedHeapReference::new(span.first_offset + slot_offset);
                let byte_len = span.class.size_class();
                let drop = if span.empty.contains(slot_index) {
                    None
                } else {
                    span.class.drop_plan()
                };
                *swept_bytes += if phase == GcPhase::Drop {
                    METADATA_STEP_BYTES
                } else {
                    byte_len.max(1)
                };

                return Ok(Some(UnreachableAllocation {
                    reference,
                    byte_len,
                    drop,
                }));
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
            let Some(block) = &store.large.blocks[block_index] else {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            };
            let block = block.read();

            // marked blocks survive this cycle
            if block.mark_epoch == mark_epoch {
                *swept_bytes += if phase == GcPhase::Drop {
                    METADATA_STEP_BYTES
                } else {
                    block.byte_len.max(1)
                };

                continue;
            }

            // skip blocks that cannot require a runtime callback during Drop
            if phase == GcPhase::Drop && (block.drop.is_none() || block.empty) {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            }

            // unmarked blocks are dead
            *swept_bytes += if phase == GcPhase::Drop {
                METADATA_STEP_BYTES
            } else {
                block.byte_len.max(1)
            };

            return Ok(Some(UnreachableAllocation {
                reference: SharedHeapReference::new(block.first_offset),
                byte_len: block.byte_len,
                drop: if block.empty { None } else { block.drop },
            }));
        }

        Ok(None)
    }

    /// Finish one completed shared collection cycle.
    fn finish_collection(&self) -> HeapResult<GcStats> {
        // collect final usage under the shared heap lock
        let mut store = self.state.write();
        let usage = self.accounting.usage();

        // build cycle stats
        let (freed_allocations, freed_bytes) = self.gc.reclaimed();
        let stats = GcStats {
            freed_allocations,
            live_allocations: usage.allocation_count,
            freed_bytes,
            allocated_bytes: usage.allocated_bytes,
            retained_bytes: usage.retained_bytes,
        };

        // cycle reset
        self.gc.close_mark_publication();
        self.gc.reset_reclaim();
        self.gc.trace_queue.clear();

        // record cycle statistics
        store.gc.record_cycle(GcCollector::Shared, stats);

        // idle publication
        self.gc.set_phase(GcPhase::Idle);
        self.gc.open_mark_publication();

        Ok(stats)
    }
}
