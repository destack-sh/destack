use crate::TraceView;
use destack_mir::TraceMap;

use crate::local::gc::{GC_METADATA_STEP_BYTES, MarkWork, Phase, ReclaimCursor};
use crate::local::storage::{HeapExtent, HeapPlace, HeapStorage};
use crate::{
    DropCursor, DropPlan, DropReference, GcAdvance, GcCollector, GcDrop, GcPhase, GcStats,
    HeapError, HeapGcStateError, HeapOperationSource, HeapReference, HeapResult, ReferenceInput,
    ReferenceRange, RootSlot, scan_references,
};

impl HeapStorage {
    /// Publish one block to an active cycle, marked and its references queued.
    pub(crate) fn publish_allocation(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.phase == Phase::Idle {
            return Ok(());
        }

        // mark the block and queue its payload
        self.mark_place(place)?;

        self.enqueue_payload_references(reference, place, 0, usize::MAX, trace_map)
    }

    /// Keep the allocations one write references for the collector.
    pub(crate) fn write_mark_barrier(
        &mut self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // scan the written range into the reusable scratch
        let scan_len = byte_len.min(self.resolve_byte_len(extent.place)? - byte_offset);
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        )?;

        // keep the referenced allocations, queueing them while a cycle runs
        let is_collecting = self.collector.phase != Phase::Idle;
        for reference in scratch.drain(..) {
            self.retain(reference)?;
            if is_collecting {
                self.enqueue_reference(reference)?;
            }
        }
        self.collector.local_reference_scratch = scratch;

        Ok(())
    }

    /// Queue the local references of one payload range into the active cycle.
    fn enqueue_payload_references(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        byte_offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // payloads without local references stay unscanned
        if !trace_map.has_local_reference() {
            return Ok(());
        }

        // resolve the scan window inside the block payload
        let place_byte_len = self.resolve_byte_len(place)?;
        let scan_len = byte_len.min(place_byte_len - byte_offset);

        // scan local reference slots into the reusable scratch
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + reference.offset();
        let result = scan_references::<HeapReference>(
            trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut scratch,
        );

        // report a failed payload scan against the owning reference
        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(())
    }

    /// Perform one full heap collection over mutable heap roots.
    pub(crate) fn collect_full<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.start_gc(roots)?;

        self.drain_gc(roots, usize::MAX, trace_view, drop)
    }

    /// Drain the active collection and return its completed stats.
    pub(crate) fn drain_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
        drop: &mut impl FnMut(GcDrop) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        // drain the cycle in bounded steps
        loop {
            match self.step_gc(roots, budget_bytes, trace_view)? {
                GcAdvance::Completed(cycle) => return Ok(cycle.stats),
                GcAdvance::Stepped(_) => continue,
                GcAdvance::Drop(request) => {
                    drop(request)?;
                    self.collector.complete_drop(request.reference)?;
                }
                GcAdvance::Started(_) => {
                    return Err(HeapError::internal(
                        "collection returned a start event during drain",
                    )
                    .into());
                }
                GcAdvance::Idle => {
                    return Err(HeapError::internal("collection made no progress").into());
                }
            }
        }
    }

    /// Start one collection.
    pub(crate) fn start_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        // reject overlapping collection work
        if self.collector.is_collecting() {
            return Err(HeapError::gc_state(HeapGcStateError::LocalGcActive).into());
        }

        // reset cycle state
        self.collector.mark_epoch += 1;
        self.collector.mark_queue.clear();
        self.collector.reclaim = ReclaimCursor::default();
        self.collector.freed_allocations = 0;
        self.collector.freed_bytes = 0;
        self.collector.phase = Phase::Mark;

        // seed initial roots
        self.seed_roots(roots)?;

        Ok(())
    }

    /// Perform bounded work for the active collection.
    pub(crate) fn step_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_view: TraceView<'_>,
    ) -> Result<GcAdvance, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.phase == Phase::Idle {
            return Ok(GcAdvance::Idle);
        }

        // advance the active phase
        match self.collector.phase {
            Phase::Idle => Ok(GcAdvance::Idle),
            Phase::Mark => {
                // roots may have changed between incremental steps
                self.seed_roots(roots)?;
                let marked_bytes = self.step_mark(budget_bytes, trace_view)?;

                // switch to reclamation when mark work drains
                if self.collector.mark_queue.is_empty() {
                    self.start_reclaim();
                    self.collector.phase = Phase::Drop;

                    // spend remaining budget dropping dead values
                    if marked_bytes < budget_bytes {
                        let remaining_bytes = budget_bytes - marked_bytes;
                        let advance = self.step_reclaim(remaining_bytes)?;

                        return Ok(advance.with_prior_work(marked_bytes));
                    }
                }

                Ok(GcAdvance::stepped(
                    GcCollector::Local,
                    GcPhase::Mark,
                    budget_bytes,
                    marked_bytes,
                ))
            }
            Phase::Drop | Phase::Sweep => {
                // drain barrier work before touching the reclamation cursors
                let marked_bytes = self.step_mark(budget_bytes, trace_view)?;
                if !self.collector.mark_queue.is_empty() || marked_bytes >= budget_bytes {
                    return Ok(GcAdvance::stepped(
                        GcCollector::Local,
                        GcPhase::Mark,
                        budget_bytes,
                        marked_bytes,
                    ));
                }

                // spend the remaining budget in the reclamation phase
                let remaining_bytes = budget_bytes - marked_bytes;
                let advance = match self.collector.phase {
                    Phase::Drop => self.step_reclaim(remaining_bytes)?,
                    Phase::Sweep => self.step_sweep(remaining_bytes)?,
                    Phase::Idle | Phase::Mark => {
                        return Err(HeapError::internal("invalid reclamation phase").into());
                    }
                };

                Ok(advance.with_prior_work(marked_bytes))
            }
        }
    }

    /// Seed the active mark queue from the explicit roots.
    fn seed_roots<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        // enqueue every reference the roots hold
        roots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_reference(reference)
        })?;

        Ok(())
    }

    /// Finish the active collection.
    fn finish_collection(&mut self) -> HeapResult<GcStats> {
        // summarize the cycle before clearing its counters
        let stats = self
            .stats_after_collection(self.collector.freed_allocations, self.collector.freed_bytes);

        // publish cycle statistics and reset collector state
        self.gc.record_cycle(GcCollector::Local, stats);
        self.collector.phase = Phase::Idle;
        self.collector.mark_queue.clear();
        self.collector.reclaim = ReclaimCursor::default();
        self.collector.freed_allocations = 0;
        self.collector.freed_bytes = 0;

        Ok(stats)
    }

    /// Start one reclamation pass over the heap tables visible to the active cycle.
    fn start_reclaim(&mut self) {
        // capture reclamation limits for a stable bounded pass
        self.collector.reclaim = ReclaimCursor {
            span_limit: self.small.spans.len(),
            large_limit: self.large.blocks.len(),
            ..ReclaimCursor::default()
        };
    }

    /// Restart the reclamation cursor for sweep.
    fn start_sweep(&mut self) {
        // rewind the cursors within the limits captured at reclamation start
        let span_limit = self.collector.reclaim.span_limit;
        let large_limit = self.collector.reclaim.large_limit;
        self.collector.reclaim = ReclaimCursor {
            span_limit,
            large_limit,
            ..ReclaimCursor::default()
        };
        self.collector.phase = Phase::Sweep;
    }

    /// Advance Drop and spend any remaining budget in sweep.
    fn step_reclaim(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        // run destructors first
        let advance = self.step_drop(budget_bytes)?;
        let drop_bytes = advance.work_bytes();

        // continue directly into sweep when Drop completed within this budget
        if self.collector.phase == Phase::Sweep && drop_bytes < budget_bytes {
            let remaining_bytes = budget_bytes - drop_bytes;

            return self
                .step_sweep(remaining_bytes)
                .map(|advance| advance.with_prior_work(drop_bytes));
        }

        Ok(advance)
    }

    /// Run Drop for unreachable values within one byte budget.
    fn step_drop(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        // preserve the active value until its destructor completes
        if self.collector.is_drop_claimed() {
            return Ok(GcAdvance::Idle);
        }

        // retire one completed allocation cursor without reclaiming storage
        self.collector.retire_completed_drop();

        // continue one repeated allocation before scanning onward
        if let Some(drop) = self.collector.continue_drop(budget_bytes)? {
            return Ok(GcAdvance::Drop(drop));
        }

        // track the byte work this step spends
        let mut work_bytes = 0usize;

        // scan spans first
        if let Some(drop) = self.step_span_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)? {
            return Ok(GcAdvance::Drop(drop));
        }

        // scan large blocks last
        if let Some(drop) =
            self.step_large_block_reclaim(Phase::Drop, budget_bytes, &mut work_bytes)?
        {
            return Ok(GcAdvance::Drop(drop));
        }

        // begin sweep only after every unreachable value completed Drop
        if self.is_reclaim_drained() {
            self.start_sweep();
        }

        Ok(GcAdvance::stepped(
            GcCollector::Local,
            GcPhase::Drop,
            budget_bytes,
            work_bytes,
        ))
    }

    /// Sweep unreachable references within one byte budget.
    fn step_sweep(&mut self, budget_bytes: usize) -> HeapResult<GcAdvance> {
        // track the byte work this step spends
        let mut swept_bytes = 0usize;

        // sweep spans first
        self.step_span_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // sweep large blocks last
        self.step_large_block_reclaim(Phase::Sweep, budget_bytes, &mut swept_bytes)?;

        // finish when all sweep cursors drain
        if self.is_reclaim_drained() {
            return self.finish_collection().map(|stats| {
                GcAdvance::completed(
                    GcCollector::Local,
                    GcPhase::Sweep,
                    budget_bytes,
                    swept_bytes,
                    stats,
                )
            });
        }

        Ok(GcAdvance::stepped(
            GcCollector::Local,
            GcPhase::Sweep,
            budget_bytes,
            swept_bytes,
        ))
    }

    /// Process unreachable span slots for one reclamation phase.
    fn step_span_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.reclaim.span_cursor < self.collector.reclaim.span_limit
        {
            // select span sweep metadata
            let span_index = self.collector.reclaim.span_cursor;
            let span = self.span_reclaim(span_index)?;

            // skip classes with no drop plan during Drop
            if phase == Phase::Drop && span.drop.is_none() {
                *work_bytes += GC_METADATA_STEP_BYTES;
                self.collector.reclaim.span_cursor += 1;
                self.collector.reclaim.slot_cursor = 0;

                continue;
            }

            // sweep slots within this span
            while *work_bytes < budget_bytes && self.collector.reclaim.slot_cursor < span.slot_count
            {
                let slot_index = self.collector.reclaim.slot_cursor;
                self.collector.reclaim.slot_cursor += 1;
                let slot = self.slot_reclaim(span_index, slot_index)?;

                // empty slots only charge metadata work
                if !slot.is_occupied {
                    *work_bytes += GC_METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                if slot.is_marked {
                    *work_bytes += if phase == Phase::Drop {
                        GC_METADATA_STEP_BYTES
                    } else {
                        span.size_class.max(1)
                    };

                    continue;
                }

                // unmarked slots are dead
                let reference =
                    HeapReference::new(span.first_offset + slot_index * span.size_class);
                if phase == Phase::Drop {
                    if let Some(drop) = span.drop
                        && !slot.is_empty
                    {
                        let cursor = DropCursor::new(
                            GcCollector::Local,
                            DropReference::Local(reference),
                            span.size_class,
                            drop,
                            *work_bytes,
                        )?;
                        let drop = self.collector.claim_drop(cursor, budget_bytes)?;

                        return Ok(Some(drop));
                    }

                    *work_bytes += GC_METADATA_STEP_BYTES;
                }
                // reclaim only after the complete Drop pass
                else if phase == Phase::Sweep {
                    self.free_swept_reference(reference, span.size_class)?;
                    *work_bytes += span.size_class.max(1);
                } else {
                    return Err(HeapError::internal("invalid span reclamation phase"));
                }
            }

            // advance after the span drains
            if self.collector.reclaim.slot_cursor >= span.slot_count {
                self.collector.reclaim.span_cursor += 1;
                self.collector.reclaim.slot_cursor = 0;
            }
        }

        Ok(None)
    }

    /// Return span reclamation metadata.
    fn span_reclaim(&self, span_index: usize) -> HeapResult<SpanReclaim> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SpanReclaim {
            first_offset: span.first_offset,
            size_class: span.class.size_class(),
            slot_count: span.slot_count,
            drop: span.class.drop_plan(),
        })
    }

    /// Return span slot reclamation state.
    fn slot_reclaim(&self, span_index: usize, slot_index: usize) -> HeapResult<SlotReclaim> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SlotReclaim {
            is_occupied: span.occupied.contains(slot_index),
            is_marked: span.mark_epoch == self.collector.mark_epoch
                && span.marked.contains(slot_index),
            is_empty: span.empty.contains(slot_index),
        })
    }

    /// Process unreachable large blocks for one reclamation phase.
    fn step_large_block_reclaim(
        &mut self,
        phase: Phase,
        budget_bytes: usize,
        work_bytes: &mut usize,
    ) -> HeapResult<Option<GcDrop>> {
        while *work_bytes < budget_bytes
            && self.collector.reclaim.large_cursor < self.collector.reclaim.large_limit
        {
            // select the next large block
            let block_index = self.collector.reclaim.large_cursor;
            self.collector.reclaim.large_cursor += 1;

            // resolve the block record
            let Some(block) = self.large.blocks.get(block_index) else {
                return Err(HeapError::internal("missing large block"));
            };

            // free block records only charge metadata work
            let Some(block) = block else {
                *work_bytes += GC_METADATA_STEP_BYTES;

                continue;
            };

            // read the block identity and size
            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            // marked blocks survive this cycle
            if block.mark_epoch == self.collector.mark_epoch {
                *work_bytes += if phase == Phase::Drop {
                    GC_METADATA_STEP_BYTES
                } else {
                    byte_len.max(1)
                };

                continue;
            }

            // run Drop without reclaiming storage
            if phase == Phase::Drop {
                if let Some(drop) = block.drop
                    && !block.empty
                {
                    let cursor = DropCursor::new(
                        GcCollector::Local,
                        DropReference::Local(reference),
                        byte_len,
                        drop,
                        *work_bytes,
                    )?;
                    let drop = self.collector.claim_drop(cursor, budget_bytes)?;

                    return Ok(Some(drop));
                }

                *work_bytes += GC_METADATA_STEP_BYTES;
            }
            // reclaim only after the complete Drop pass
            else if phase == Phase::Sweep {
                self.free_swept_reference(reference, byte_len)?;
                *work_bytes += byte_len.max(1);
            } else {
                return Err(HeapError::internal("invalid large block reclamation phase"));
            }
        }

        Ok(None)
    }

    /// Return whether the active reclamation cursor is drained.
    fn is_reclaim_drained(&self) -> bool {
        self.collector.reclaim.span_cursor >= self.collector.reclaim.span_limit
            && self.collector.reclaim.large_cursor >= self.collector.reclaim.large_limit
    }

    /// Free one unreachable block discovered by sweep.
    fn free_swept_reference(
        &mut self,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        // free physical storage
        self.free(reference)
            .map_err(|error| HeapError::free_failed(reference, error))?;

        // record cycle accounting
        self.collector.freed_allocations += 1;
        self.collector.freed_bytes += byte_len as u64;

        Ok(())
    }

    /// Mark reachable heap references within one byte budget.
    fn step_mark(&mut self, budget_bytes: usize, trace_view: TraceView<'_>) -> HeapResult<usize> {
        // track the byte work this step spends
        let mut marked_bytes = 0usize;

        // trace bounded mark work
        while marked_bytes < budget_bytes {
            // claim the next bounded mark item
            let Some(work) = self.collector.mark_queue.pop() else {
                break;
            };

            match work {
                // large blocks are scanned page by page
                MarkWork::LargeRange { reference, start } => {
                    marked_bytes += self.trace_large_range(reference, start, trace_view)?;

                    continue;
                }

                // span slots are scanned as one mark item
                MarkWork::Reference(reference) => {
                    marked_bytes += self.trace_reference(reference, trace_view)?;
                }
            }
        }

        Ok(marked_bytes)
    }

    /// Trace one local heap reference.
    fn trace_reference(
        &mut self,
        reference: HeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        // resolve the referenced block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // scan local references inside the payload into the reusable scratch
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        let trace_result = self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::All,
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        );

        // report a failed payload scan against the owning reference
        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        Ok(extent.byte_len.max(1))
    }

    /// Trace one memory page of one local large block.
    fn trace_large_range(
        &mut self,
        reference: HeapReference,
        start: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<usize> {
        // resolve and verify the large block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        let HeapPlace::LargeBlock(_) = extent.place else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // scan only payloads that hold local references inside the block
        let has_local_reference = self
            .has_reference::<HeapReference>(extent.place, trace_view)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;
        if !has_local_reference || start >= extent.byte_len {
            return Ok(0);
        }

        // scan at most one memory page into the reusable scratch
        let range_len = self.page_size_bytes().min(extent.byte_len - start);
        let mut scratch = std::mem::take(&mut self.collector.local_reference_scratch);
        let base_address = self.memory.base_address() + extent.base.offset();
        let trace_result = self.visit_references::<HeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut |reference| {
                scratch.push(reference);

                Ok(())
            },
        );

        // report a failed payload scan against the owning reference
        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue the discovered references
        for reference in scratch.drain(..) {
            self.enqueue_reference(reference)?;
        }
        self.collector.local_reference_scratch = scratch;

        // continue this large block on a later step
        let next_start = start + range_len;
        if next_start < extent.byte_len {
            self.collector.mark_queue.push(MarkWork::LargeRange {
                reference,
                start: next_start,
            });
        }

        Ok(range_len)
    }

    /// Queue one reference after marking it.
    fn enqueue_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        // skip nullish and constant references
        if reference.is_nullish() || self.is_constant(reference) {
            return Ok(());
        }

        // mark the referenced block before queueing scan work, an interior address by its block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        let reference = HeapReference::new(reference.offset() - extent.byte_offset);

        // queue scan work for freshly marked blocks
        if self.mark_place(extent.place)? {
            // slice large blocks to keep each mark step bounded
            if matches!(extent.place, HeapPlace::LargeBlock(_)) {
                self.collector.mark_queue.push(MarkWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller blocks are one mark item
            self.collector
                .mark_queue
                .push(MarkWork::Reference(reference));
        }

        Ok(())
    }

    /// Return whether one heap place is marked in the active cycle.
    pub(super) fn is_marked_place(&self, place: HeapPlace) -> HeapResult<bool> {
        // dispatch by physical heap place
        match place {
            HeapPlace::Slot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(span.mark_epoch == self.collector.mark_epoch
                    && span.marked.contains(slot.slot_index()))
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                Ok(block.mark_epoch == self.collector.mark_epoch)
            }
        }
    }

    /// Mark one heap place and return whether this was the first mark.
    pub(crate) fn mark_place(&mut self, place: HeapPlace) -> HeapResult<bool> {
        // skip places already marked
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        // mark by physical heap place
        match place {
            HeapPlace::Slot(slot) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.ensure_mark_epoch(mark_epoch);
                span.marked.set(slot.slot_index());
            }
            HeapPlace::LargeBlock(block_id) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.mark_epoch = mark_epoch;
            }
        }

        Ok(true)
    }

    /// Build one GC statistics snapshot after one collection pass.
    pub(super) fn stats_after_collection(
        &self,
        freed_allocations: usize,
        freed_bytes: u64,
    ) -> GcStats {
        GcStats {
            freed_allocations,
            live_allocations: self.usage.allocation_count(),
            freed_bytes,
            allocated_bytes: self.usage.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }
}

/// Span metadata needed by reclamation.
struct SpanReclaim {
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of slots to sweep.
    slot_count: usize,
    /// The drop plan for each managed slot.
    drop: Option<DropPlan>,
}

/// Span slot state needed by reclamation.
struct SlotReclaim {
    /// Whether the slot is currently occupied.
    is_occupied: bool,
    /// Whether the slot is marked in the active cycle.
    is_marked: bool,
    /// Whether the slot's values moved out before its release.
    is_empty: bool,
}
