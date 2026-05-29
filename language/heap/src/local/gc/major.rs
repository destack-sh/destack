use destack_mir::{TraceMap, TraceTable};

use crate::local::space::{
    GcKind, GcStats, HeapExtent, HeapSpace, HeapStorage, LocalGcPhase, LocalTraceWork,
    MajorSweepCursor,
};
use crate::{
    GcProgress, HeapError, HeapGcStateError, HeapOperationSource, HeapReference, HeapResult,
    ReferenceInput, ReferenceRange, RootSlot, scan_references,
};

/// The budget charged for one metadata-only sweep step.
const METADATA_STEP_BYTES: usize = 1;
/// The number of metadata bits skipped by one bitmap word scan.
const METADATA_WORD_BITS: usize = u64::BITS as usize;

impl HeapSpace {
    /// Return whether one local major collection is active.
    pub(crate) fn major_gc_active(&self) -> bool {
        self.collector.major_phase != LocalGcPhase::Idle
    }

    /// Publish one block to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        storage: HeapStorage,
        trace_map: &TraceMap,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == LocalGcPhase::Idle {
            return Ok(());
        }

        self.mark_place(storage)?;

        self.enqueue_location_heap_references(reference, storage, 0, usize::MAX, trace_map)
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
        if self.collector.major_phase == LocalGcPhase::Idle {
            return Ok(());
        }

        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;

        self.enqueue_location_heap_references(
            extent.base,
            extent.storage,
            byte_offset,
            byte_len,
            &trace_map,
        )
    }

    /// Queue local references from one heap payload range into the active major cycle.
    fn enqueue_location_heap_references(
        &mut self,
        reference: HeapReference,
        storage: HeapStorage,
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
        let mut references = Vec::new();

        // scan local reference slots in mapped heap memory
        let base_address = self.mapping.base_address() + extent.base.offset();
        let result = scan_references::<HeapReference>(
            trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, scan_len),
            &mut references,
        );

        if let Err(error) = result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue after scanner borrows have ended
        for reference in references {
            self.enqueue_major_reference(reference)?;
        }

        Ok(())
    }

    /// Perform one full heap collection over mutable heap roots.
    pub fn collect_full<E>(
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
        self.collector.major_phase = LocalGcPhase::Mark;

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
        if budget_bytes == 0 || self.collector.major_phase == LocalGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        // phase work
        match self.collector.major_phase {
            LocalGcPhase::Idle => Ok(GcProgress::Idle),
            LocalGcPhase::Mark => {
                // roots may have changed between incremental steps
                self.seed_major_roots(roots)?;
                let marked_bytes =
                    self.mark_reachable_references_step(budget_bytes, trace_table)?;

                // switch to sweep when mark work drains
                if self.collector.major_queue.is_empty() {
                    self.start_major_sweep();
                    self.collector.major_phase = LocalGcPhase::Sweep;

                    // spend remaining budget in the sweep phase
                    if marked_bytes < budget_bytes {
                        let remaining_bytes = budget_bytes - marked_bytes;

                        return Ok(self.sweep_unreachable_references_step(remaining_bytes)?);
                    }
                }

                Ok(GcProgress::Active)
            }
            LocalGcPhase::Sweep => {
                let marked_bytes =
                    self.mark_reachable_references_step(budget_bytes, trace_table)?;
                if !self.collector.major_queue.is_empty() {
                    return Ok(GcProgress::Active);
                }
                if marked_bytes >= budget_bytes {
                    return Ok(GcProgress::Active);
                }

                let remaining_bytes = budget_bytes - marked_bytes;

                Ok(self.sweep_unreachable_references_step(remaining_bytes)?)
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
        self.collector.major_phase = LocalGcPhase::Idle;
        self.collector.major_queue.clear();
        self.collector.major_sweep = MajorSweepCursor::default();
        self.collector.major_freed_allocations = 0;
        self.collector.major_freed_bytes = 0;

        Ok(stats)
    }

    /// Start sweeping over the heap tables visible to the active major cycle.
    fn start_major_sweep(&mut self) {
        self.flush_young_cursor();
        self.collector.major_sweep = MajorSweepCursor {
            small_span_limit: self.small.spans.len(),
            large_limit: self.large.blocks.len(),
            ..MajorSweepCursor::default()
        };
    }

    /// Sweep unreachable references within one byte budget.
    fn sweep_unreachable_references_step(&mut self, budget_bytes: usize) -> HeapResult<GcProgress> {
        let mut swept_bytes = 0usize;

        self.sweep_major_young_ranges_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_young_spans_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_small_spans_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_large_blocks_step(budget_bytes, &mut swept_bytes)?;

        if self.major_sweep_drained() {
            return self.finish_major_gc().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Sweep unreachable young range blocks within one byte budget.
    fn sweep_major_young_ranges_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_range_cursor < self.young.ranges.len()
        {
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

            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::Internal {
                    context: "live young range missing during major sweep",
                });
            };
            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            if self.young.marked.contains(block_index) {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            self.free_swept_reference(reference, byte_len)?;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Sweep unreachable young span slots within one byte budget.
    fn sweep_major_young_spans_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_span_cursor < self.young.spans.len()
        {
            let span_index = self.collector.major_sweep.young_span_cursor;
            let Some(span) = self.young.span(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };
            let reserved_slots = self
                .young
                .span_reserved_slot_count(span_index)
                .unwrap_or_else(|| span.slot_count());

            while *swept_bytes < budget_bytes
                && self.collector.major_sweep.young_slot_cursor < reserved_slots
            {
                let slot_index = self.collector.major_sweep.young_slot_cursor;
                self.collector.major_sweep.young_slot_cursor += 1;

                let Some(bits) = self.young.span_bits_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };
                if bits.freed.contains(slot_index) {
                    *swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                if bits.marked.contains(slot_index) {
                    *swept_bytes += span.class.size_class.max(1);

                    continue;
                }

                bits.freed.set(slot_index);
                bits.marked.clear(slot_index);
                let reference = HeapReference::new(span.slot_offset(slot_index));
                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(span.class.size_class);
                self.collector.major_freed_allocations += 1;
                self.collector.major_freed_bytes += span.class.size_class as u64;
                *swept_bytes += span.class.size_class.max(1);
            }

            if self.collector.major_sweep.young_slot_cursor >= reserved_slots {
                self.collector.major_sweep.young_span_cursor += 1;
                self.collector.major_sweep.young_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Sweep unreachable small-span slots within one byte budget.
    fn sweep_major_small_spans_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.small_span_cursor
                < self.collector.major_sweep.small_span_limit
        {
            let span_index = self.collector.major_sweep.small_span_cursor;
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_count = span.slot_count;
            let size_class = span.class.size_class;

            while *swept_bytes < budget_bytes
                && self.collector.major_sweep.small_slot_cursor < slot_count
            {
                let slot_index = self.collector.major_sweep.small_slot_cursor;
                self.collector.major_sweep.small_slot_cursor += 1;

                let Some(span) = self.span(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };
                if !span.occupied.contains(slot_index) {
                    *swept_bytes += METADATA_STEP_BYTES;

                    continue;
                }

                let is_marked = span.mark_epoch == self.collector.mark_epoch
                    && span.marked.contains(slot_index);
                if is_marked {
                    *swept_bytes += size_class.max(1);

                    continue;
                }

                let reference = HeapReference::new(span.first_offset + slot_index * size_class);
                self.free_swept_reference(reference, size_class)?;
                *swept_bytes += size_class.max(1);
            }

            if self.collector.major_sweep.small_slot_cursor >= slot_count {
                self.collector.major_sweep.small_span_cursor += 1;
                self.collector.major_sweep.small_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Sweep unreachable large blocks within one byte budget.
    fn sweep_major_large_blocks_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.large_cursor < self.collector.major_sweep.large_limit
        {
            let block_index = self.collector.major_sweep.large_cursor;
            self.collector.major_sweep.large_cursor += 1;

            let Some(block) = self.large.blocks.get(block_index) else {
                return Err(HeapError::internal("missing large block"));
            };
            if !block.is_live {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            }

            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;
            if block.mark_epoch == self.collector.mark_epoch {
                *swept_bytes += byte_len.max(1);

                continue;
            }

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
        self.free(reference)
            .map_err(|error| HeapError::free_failed(reference, error))?;
        self.collector.major_freed_allocations += 1;
        self.collector.major_freed_bytes += byte_len as u64;

        Ok(())
    }

    /// Mark reachable heap references within one byte budget.
    fn mark_reachable_references_step(
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
                LocalTraceWork::LargeRange { reference, start } => {
                    marked_bytes += self.trace_large_range(reference, start, trace_table)?;

                    continue;
                }

                // small and young blocks are scanned as one mark item
                LocalTraceWork::Reference(reference) => {
                    let Some(extent) = self.resolve_extent(reference) else {
                        return Err(HeapError::invalid_heap_reference(reference));
                    };

                    marked_bytes += extent.byte_len.max(1);

                    // read layout side metadata for this payload
                    let trace_map = self
                        .trace_map_for_place(extent.storage, trace_table)
                        .map_err(|error| {
                            HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
                        })?;

                    let mut references = Vec::new();

                    // scan every local reference discovered in this payload
                    let base_address = self.mapping.base_address() + extent.base.offset();
                    let trace_result = scan_references::<HeapReference>(
                        &trace_map,
                        ReferenceInput::mapped(base_address),
                        ReferenceRange::All,
                        &mut references,
                    );

                    if let Err(error) = trace_result {
                        return Err(HeapError::scan_failed(
                            HeapOperationSource::Reference(reference),
                            error,
                        ));
                    }

                    // validate and enqueue after the read borrow has ended
                    for reference in references {
                        if !reference.is_null() && self.resolve_extent(reference).is_none() {
                            return Err(HeapError::invalid_heap_reference(reference));
                        }

                        self.enqueue_major_reference(reference)?;
                    }
                }
            }
        }

        Ok(marked_bytes)
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
        let HeapStorage::LargeBlock(_) = extent.storage else {
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
        let mut references = Vec::new();
        let base_address = self.mapping.base_address() + extent.base.offset();
        let trace_result = scan_references::<HeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(start, range_len),
            &mut references,
        );

        if let Err(error) = trace_result {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Reference(reference),
                error,
            ));
        }

        // enqueue discovered local references after the read borrow ends
        for reference in references {
            if !reference.is_null() && self.resolve_extent(reference).is_none() {
                return Err(HeapError::invalid_heap_reference(reference));
            }

            self.enqueue_major_reference(reference)?;
        }

        // continue this large block on a later step
        let next_start = start + range_len;
        if next_start < extent.byte_len {
            self.collector.major_queue.push(LocalTraceWork::LargeRange {
                reference,
                start: next_start,
            });
        }

        Ok(range_len)
    }

    /// Queue one major collection reference after marking it.
    fn enqueue_major_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
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
            if matches!(extent.storage, HeapStorage::LargeBlock(_)) {
                self.collector.major_queue.push(LocalTraceWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller blocks are one mark item
            self.collector
                .major_queue
                .push(LocalTraceWork::Reference(reference));
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
    pub(super) fn is_marked_place(&self, storage: HeapStorage) -> HeapResult<bool> {
        // dispatch by physical heap storage
        match storage {
            HeapStorage::YoungRange { first_offset } => {
                let Some((block_index, _allocation)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::internal("missing young range"));
                };

                Ok(self.young.marked.contains(block_index))
            }
            HeapStorage::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(bits.marked.contains(slot.slot_index()))
            }
            HeapStorage::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(span.mark_epoch == self.collector.mark_epoch
                    && span.marked.contains(slot.slot_index()))
            }
            HeapStorage::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                Ok(block.mark_epoch == self.collector.mark_epoch)
            }
        }
    }

    /// Mark one heap storage and return whether this was the first mark.
    pub(super) fn mark_place(&mut self, storage: HeapStorage) -> HeapResult<bool> {
        // skip already marked storages
        if self.is_marked_place(storage)? {
            return Ok(false);
        }

        // mark by physical heap storage
        match storage {
            HeapStorage::YoungRange { first_offset } => {
                let Some((block_index, _allocation)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::internal("missing young range"));
                };

                self.young.marked.set(block_index);
            }
            HeapStorage::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.marked.set(slot.slot_index());
            }
            HeapStorage::MatureSlot(slot) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.ensure_mark_epoch(mark_epoch);
                span.marked.set(slot.slot_index());
            }
            HeapStorage::LargeBlock(block_id) => {
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
