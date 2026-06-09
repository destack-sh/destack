use destack_mir::{TraceMap, TraceTable};

use crate::local::gc::{MajorSweepCursor, MarkWork, Phase};
use crate::local::storage::{GcKind, GcStats, HeapExtent, HeapPlace, HeapStorage};
use crate::{
    GcProgress, HeapError, HeapGcStateError, HeapOperationSource, HeapReference, HeapResult,
    ReferenceInput, ReferenceRange, RootSlot, visit_references,
};

/// The budget charged for one metadata-only sweep step.
const METADATA_STEP_BYTES: usize = 1;
/// The number of metadata bits skipped by one bitmap word scan.
const METADATA_WORD_BITS: usize = u64::BITS as usize;

impl HeapStorage {
    /// Return whether one local major collection is active.
    pub(crate) fn major_gc_active(&self) -> bool {
        self.collector.major_phase != Phase::Idle
    }

    /// Publish one block to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        storage: HeapPlace,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == Phase::Idle {
            return Ok(());
        }

        self.mark_place(storage)?;

        self.enqueue_payload_references(reference, storage, 0, usize::MAX, trace_map)
    }

    /// Queue local references written into one active local major cycle.
    pub(crate) fn write_major_barrier(
        &mut self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == Phase::Idle {
            return Ok(());
        }

        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;

        self.enqueue_payload_references(
            extent.base,
            extent.storage,
            byte_offset,
            byte_len,
            &trace_map,
        )
    }

    /// Queue local references from one heap payload range into the active major cycle.
    fn enqueue_payload_references(
        &mut self,
        reference: HeapReference,
        storage: HeapPlace,
        byte_offset: usize,
        byte_len: usize,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        if !trace_map.has_local_reference() {
            return Ok(());
        }

        // resolve the scan window inside the block payload
        let extent = HeapExtent {
            storage,
            base: reference,
            byte_offset: 0,
            byte_len: self.byte_len_for_place(storage)?,
        };
        let scan_len = byte_len.min(extent.byte_len - byte_offset);

        // scan local reference slots in mapped heap memory
        let base_address = self.mapping.base_address() + extent.base.offset();
        let result = visit_references::<HeapReference>(
            trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut |reference| self.enqueue_major_reference(reference),
        );

        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        Ok(())
    }

    /// Perform one full heap collection over mutable heap roots.
    pub(crate) fn collect_full<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_table: &TraceTable,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        // collect the nursery first so full collection sees mature references
        let _minor = self.collect_minor(roots, trace_table)?;

        self.start_major_gc(roots)?;

        // drain the active major cycle synchronously
        loop {
            match self.step_major_gc(roots, usize::MAX, trace_table)? {
                GcProgress::Complete(stats) => return Ok(stats),
                GcProgress::Active => continue,
                GcProgress::Idle => {
                    return Err(HeapError::Internal {
                        context: "local full collection made no progress",
                    }
                    .into());
                }
            }
        }
    }

    /// Start one local major collection.
    pub(crate) fn start_major_gc<E>(
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

        // publish active span accounting before tracing
        self.flush_young_cursor();

        // reset cycle state
        self.collector.mark_epoch += 1;
        self.clear_young_mark_bits();
        self.collector.major_queue.clear();
        self.collector.major_sweep = MajorSweepCursor::default();
        self.collector.major_freed_allocations = 0;
        self.collector.major_freed_bytes = 0;
        self.collector.major_phase = Phase::Mark;

        // seed initial roots
        self.seed_major_roots(roots)?;

        Ok(())
    }

    /// Perform bounded work for one active local major collection.
    pub(crate) fn step_major_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.major_phase == Phase::Idle {
            return Ok(GcProgress::Idle);
        }

        // phase work
        match self.collector.major_phase {
            Phase::Idle => Ok(GcProgress::Idle),
            Phase::Mark => {
                // roots may have changed between incremental steps
                self.seed_major_roots(roots)?;
                let marked_bytes = self.step_reachable_reference_mark(budget_bytes, trace_table)?;

                // switch to sweep when mark work drains
                if self.collector.major_queue.is_empty() {
                    self.start_major_sweep();
                    self.collector.major_phase = Phase::Sweep;

                    // spend remaining budget in the sweep phase
                    if marked_bytes < budget_bytes {
                        let remaining_bytes = budget_bytes - marked_bytes;

                        return Ok(self.step_unreachable_reference_sweep(remaining_bytes)?);
                    }
                }

                Ok(GcProgress::Active)
            }
            Phase::Sweep => {
                let marked_bytes = self.step_reachable_reference_mark(budget_bytes, trace_table)?;
                if !self.collector.major_queue.is_empty() {
                    return Ok(GcProgress::Active);
                }
                if marked_bytes >= budget_bytes {
                    return Ok(GcProgress::Active);
                }

                let remaining_bytes = budget_bytes - marked_bytes;

                Ok(self.step_unreachable_reference_sweep(remaining_bytes)?)
            }
        }
    }

    /// Seed the active major trace queue from explicit roots and pins.
    fn seed_major_roots<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        // root slots
        roots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_major_reference(reference)
        })?;

        // pins
        let pins = std::mem::take(&mut self.collector.pins);
        for reference in pins.references() {
            self.enqueue_major_reference(reference)?;
        }
        self.collector.pins = pins;

        Ok(())
    }

    /// Finish the active local major collection.
    fn finish_major_gc(&mut self) -> HeapResult<GcStats> {
        let stats = self.stats_after_collection(
            self.collector.major_freed_allocations,
            self.collector.major_freed_bytes,
        );

        // publish cycle statistics and reset collector state
        self.gc.record_cycle(GcKind::Full, stats);
        self.collector.major_phase = Phase::Idle;
        self.collector.major_queue.clear();
        self.collector.major_sweep = MajorSweepCursor::default();
        self.collector.major_freed_allocations = 0;
        self.collector.major_freed_bytes = 0;

        Ok(stats)
    }

    /// Start sweeping over the heap tables visible to the active major cycle.
    fn start_major_sweep(&mut self) {
        // publish active span accounting before cursor bounds are captured
        self.flush_young_cursor();

        // capture sweep limits for a stable bounded pass
        self.collector.major_sweep = MajorSweepCursor {
            small_span_limit: self.small.spans.len(),
            large_limit: self.large.blocks.len(),
            ..MajorSweepCursor::default()
        };
    }

    /// Sweep unreachable references within one byte budget.
    fn step_unreachable_reference_sweep(&mut self, budget_bytes: usize) -> HeapResult<GcProgress> {
        let mut swept_bytes = 0usize;

        // sweep young variable ranges first
        self.step_major_young_range_sweep(budget_bytes, &mut swept_bytes)?;

        // sweep young fixed spans with remaining budget
        self.step_major_young_span_sweep(budget_bytes, &mut swept_bytes)?;

        // sweep mature small spans with remaining budget
        self.step_major_small_span_sweep(budget_bytes, &mut swept_bytes)?;

        // sweep mature large blocks last
        self.step_major_large_block_sweep(budget_bytes, &mut swept_bytes)?;

        // finish when all sweep cursors drain
        if self.major_sweep_drained() {
            return self.finish_major_gc().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Sweep unreachable young range blocks within one byte budget.
    fn step_major_young_range_sweep(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_range_cursor < self.young.ranges.len()
        {
            // skip clear bitmap words as charged metadata work
            let Some(block_index) = self
                .young
                .live
                .first_set_from(self.collector.major_sweep.young_range_cursor)
            else {
                self.collector.major_sweep.young_range_cursor = charge_bitmap_skip(
                    self.collector.major_sweep.young_range_cursor,
                    self.young.ranges.len(),
                    budget_bytes,
                    swept_bytes,
                );

                return Ok(());
            };

            // charge skipped dead range bits
            self.collector.major_sweep.young_range_cursor = charge_bitmap_skip(
                self.collector.major_sweep.young_range_cursor,
                block_index,
                budget_bytes,
                swept_bytes,
            );
            if *swept_bytes >= budget_bytes {
                return Ok(());
            }
            self.collector.major_sweep.young_range_cursor = block_index + 1;

            // resolve the live range selected by the bitmap
            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::Internal {
                    context: "live young range missing during major sweep",
                });
            };
            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            // marked ranges survive this cycle
            if self.young.marked.contains(block_index) {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            // unmarked ranges are dead
            self.free_swept_reference(reference, byte_len)?;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Sweep unreachable young span slots within one byte budget.
    fn step_major_young_span_sweep(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_span_cursor < self.young.spans.len()
        {
            // select the active span and reserved slot limit
            let span_index = self.collector.major_sweep.young_span_cursor;
            let span = self.select_major_young_span_sweep(span_index)?;

            // sweep slots within this young span
            while *swept_bytes < budget_bytes
                && self.collector.major_sweep.young_slot_cursor < span.reserved_slots
            {
                let slot_index = self.collector.major_sweep.young_slot_cursor;
                self.collector.major_sweep.young_slot_cursor += 1;

                // already freed slots only charge metadata work
                let Some(bits) = self.young.span_bits_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };
                if bits.freed.contains(slot_index) {
                    *swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                if bits.marked.contains(slot_index) {
                    *swept_bytes += span.size_class.max(1);

                    continue;
                }

                // unmarked slots are dead
                bits.freed.set(slot_index);
                bits.marked.clear(slot_index);
                let reference =
                    HeapReference::new(span.first_offset + slot_index * span.size_class);
                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(span.size_class);
                self.collector.major_freed_allocations += 1;
                self.collector.major_freed_bytes += span.size_class as u64;
                *swept_bytes += span.size_class.max(1);
            }

            // advance after all reserved slots drain
            if self.collector.major_sweep.young_slot_cursor >= span.reserved_slots {
                self.collector.major_sweep.young_span_cursor += 1;
                self.collector.major_sweep.young_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Select young span sweep metadata without cloning span bitmaps.
    fn select_major_young_span_sweep(&self, span_index: usize) -> HeapResult<YoungSpanSweep> {
        let Some(span) = self.young.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        let reserved_slots = self
            .young
            .span_reserved_slot_count(span_index)
            .unwrap_or_else(|| span.slot_count());

        Ok(YoungSpanSweep {
            first_offset: span.first_offset,
            size_class: span.class.size_class,
            reserved_slots,
        })
    }

    /// Sweep unreachable small-span slots within one byte budget.
    fn step_major_small_span_sweep(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.small_span_cursor
                < self.collector.major_sweep.small_span_limit
        {
            // select mature span sweep metadata
            let span_index = self.collector.major_sweep.small_span_cursor;
            let span = self.select_major_small_span_sweep(span_index)?;

            // sweep slots within this mature span
            while *swept_bytes < budget_bytes
                && self.collector.major_sweep.small_slot_cursor < span.slot_count
            {
                let slot_index = self.collector.major_sweep.small_slot_cursor;
                self.collector.major_sweep.small_slot_cursor += 1;
                let slot = self.select_major_small_slot_sweep(span_index, slot_index)?;

                // empty slots only charge metadata work
                if !slot.is_occupied {
                    *swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                // marked slots survive this cycle
                if slot.is_marked {
                    *swept_bytes += span.size_class.max(1);

                    continue;
                }

                // unmarked slots are dead
                let reference =
                    HeapReference::new(span.first_offset + slot_index * span.size_class);
                self.free_swept_reference(reference, span.size_class)?;
                *swept_bytes += span.size_class.max(1);
            }

            // advance after the span drains
            if self.collector.major_sweep.small_slot_cursor >= span.slot_count {
                self.collector.major_sweep.small_span_cursor += 1;
                self.collector.major_sweep.small_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Select mature small span sweep metadata.
    fn select_major_small_span_sweep(&self, span_index: usize) -> HeapResult<SmallSpanSweep> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SmallSpanSweep {
            first_offset: span.first_offset,
            size_class: span.class.size_class,
            slot_count: span.slot_count,
        })
    }

    /// Select mature small slot sweep state.
    fn select_major_small_slot_sweep(
        &self,
        span_index: usize,
        slot_index: usize,
    ) -> HeapResult<SmallSlotSweep> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        Ok(SmallSlotSweep {
            is_occupied: span.occupied.contains(slot_index),
            is_marked: span.mark_epoch == self.collector.mark_epoch
                && span.marked.contains(slot_index),
        })
    }

    /// Sweep unreachable large blocks within one byte budget.
    fn step_major_large_block_sweep(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.large_cursor < self.collector.major_sweep.large_limit
        {
            // select the next large block
            let block_index = self.collector.major_sweep.large_cursor;
            self.collector.major_sweep.large_cursor += 1;

            // inactive blocks only charge metadata work
            let Some(block) = self.large.blocks.get(block_index) else {
                return Err(HeapError::internal("missing large block"));
            };
            if !block.is_live {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            }

            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            // marked blocks survive this cycle
            if block.mark_epoch == self.collector.mark_epoch {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            // unmarked blocks are dead
            self.free_swept_reference(reference, byte_len)?;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Return whether every active major sweep cursor is drained.
    fn major_sweep_drained(&self) -> bool {
        self.collector.major_sweep.young_range_cursor >= self.young.ranges.len()
            && self.collector.major_sweep.young_span_cursor >= self.young.spans.len()
            && self.collector.major_sweep.small_span_cursor
                >= self.collector.major_sweep.small_span_limit
            && self.collector.major_sweep.large_cursor >= self.collector.major_sweep.large_limit
    }

    /// Free one unreachable block discovered by major sweep.
    fn free_swept_reference(
        &mut self,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        // free physical storage
        self.free(reference)
            .map_err(|error| HeapError::free_failed(reference, error))?;

        // record cycle accounting
        self.collector.major_freed_allocations += 1;
        self.collector.major_freed_bytes += byte_len as u64;

        Ok(())
    }

    /// Mark reachable heap references within one byte budget.
    fn step_reachable_reference_mark(
        &mut self,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;

        // trace bounded mark work
        while marked_bytes < budget_bytes {
            // claim the next bounded mark item
            let Some(work) = self.collector.major_queue.pop() else {
                break;
            };

            match work {
                // large blocks are scanned page by page
                MarkWork::LargeRange { reference, start } => {
                    marked_bytes += self.trace_large_range(reference, start, trace_table)?;

                    continue;
                }

                // small and young blocks are scanned as one mark item
                MarkWork::Reference(reference) => {
                    marked_bytes += self.trace_reference(reference, trace_table)?;
                }
            }
        }

        Ok(marked_bytes)
    }

    /// Trace one local heap reference.
    fn trace_reference(
        &mut self,
        reference: HeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // resolve the referenced block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // resolve the payload trace map
        let trace_map = self
            .trace_map_for_place(extent.storage, trace_table)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;

        // scan local references inside the payload
        let base_address = self.mapping.base_address() + extent.base.offset();
        let trace_result = visit_references::<HeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::All,
            &mut |reference| {
                if !reference.is_null() && self.resolve_extent(reference).is_none() {
                    return Err(HeapError::invalid_heap_reference(reference));
                }

                self.enqueue_major_reference(reference)
            },
        );

        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        Ok(extent.byte_len.max(1))
    }

    /// Trace one page-sized range from one local large block.
    fn trace_large_range(
        &mut self,
        reference: HeapReference,
        start: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        // resolve and verify the large block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        let HeapPlace::LargeBlock(_) = extent.storage else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // skip empty ranges and noscan payloads
        let trace_map = self
            .trace_map_for_place(extent.storage, trace_table)
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
            })?;
        if !trace_map.has_local_reference() || start >= extent.byte_len {
            return Ok(0);
        }

        // scan at most one allocator page
        let range_len = self
            .allocator()
            .page_size_bytes()
            .min(extent.byte_len - start);
        let base_address = self.mapping.base_address() + extent.base.offset();
        let trace_result = visit_references::<HeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut |reference| {
                if !reference.is_null() && self.resolve_extent(reference).is_none() {
                    return Err(HeapError::invalid_heap_reference(reference));
                }

                self.enqueue_major_reference(reference)
            },
        );

        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // continue this large block on a later step
        let next_start = start + range_len;
        if next_start < extent.byte_len {
            self.collector.major_queue.push(MarkWork::LargeRange {
                reference,
                start: next_start,
            });
        }

        Ok(range_len)
    }

    /// Queue one major collection reference after marking it.
    fn enqueue_major_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        // null references are not heap roots
        if reference.is_null() {
            return Ok(());
        }

        // mark the referenced block before queueing scan work
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        if self.mark_place(extent.storage)? {
            // marked noscan blocks need no queued scan work
            // large blocks are sliced to keep major steps bounded
            if matches!(extent.storage, HeapPlace::LargeBlock(_)) {
                self.collector.major_queue.push(MarkWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller blocks are one mark item
            self.collector
                .major_queue
                .push(MarkWork::Reference(reference));
        }

        Ok(())
    }

    /// Clear every young-space mark bit.
    pub(super) fn clear_young_mark_bits(&mut self) {
        self.young.marked.clear_all();
        for bits in &mut self.young.span_bits {
            bits.marked.clear_all();
        }
    }

    /// Return whether one heap storage is marked in the active cycle.
    pub(super) fn is_marked_place(&self, storage: HeapPlace) -> HeapResult<bool> {
        // dispatch by physical heap storage
        match storage {
            HeapPlace::YoungRange { first_offset } => {
                let Some(range) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                Ok(self.young.marked.contains(range.index))
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(bits.marked.contains(slot.slot_index()))
            }
            HeapPlace::MatureSlot(slot) => {
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

    /// Mark one heap storage and return whether this was the first mark.
    pub(super) fn mark_place(&mut self, storage: HeapPlace) -> HeapResult<bool> {
        // skip already marked storages
        if self.is_marked_place(storage)? {
            return Ok(false);
        }

        // mark by physical heap storage
        match storage {
            HeapPlace::YoungRange { first_offset } => {
                let Some(range) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                self.young.marked.set(range.index);
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.marked.set(slot.slot_index());
            }
            HeapPlace::MatureSlot(slot) => {
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

/// Young span metadata selected for major sweep.
struct YoungSpanSweep {
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of reserved slots to sweep.
    reserved_slots: usize,
}

/// Mature small span metadata selected for major sweep.
struct SmallSpanSweep {
    /// The span byte offset within heap storage.
    first_offset: usize,
    /// The span size class in bytes.
    size_class: usize,
    /// The number of slots to sweep.
    slot_count: usize,
}

/// Mature small slot state selected for major sweep.
struct SmallSlotSweep {
    /// Whether the slot is currently occupied.
    is_occupied: bool,
    /// Whether the slot is marked in the active major cycle.
    is_marked: bool,
}

/// Charge bitmap metadata work and return the cursor reached.
fn charge_bitmap_skip(
    start: usize,
    end: usize,
    budget_bytes: usize,
    swept_bytes: &mut usize,
) -> usize {
    if start >= end {
        return end;
    }

    let skipped_bits = end - start;
    let skipped_words = skipped_bits.div_ceil(METADATA_WORD_BITS);
    let remaining_budget = budget_bytes - *swept_bytes;
    if skipped_words <= remaining_budget {
        *swept_bytes += skipped_words;

        return end;
    }

    *swept_bytes = budget_bytes;

    start + remaining_budget * METADATA_WORD_BITS
}
