use destack_mir::TraceTable;

use crate::local::gc::{DirtyExtent, GC_METADATA_STEP_BYTES, GC_METADATA_WORD_BITS, Phase};
use crate::local::storage::{GcKind, GcStats, HeapPlace, HeapStorage, LargeBlockId, YoungCursor};
use crate::{
    GcProgress, HeapError, HeapGcStateError, HeapOperationSource, HeapReference, HeapResult,
    ReferenceInput, ReferenceRange, RootSlot, TraceQueue, scan_references,
};

impl HeapStorage {
    /// Perform one young-generation collection over mutable heap roots.
    pub(crate) fn collect_minor<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        trace_table: &TraceTable,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.start_young_gc()?;

        loop {
            match self.step_young_gc(roots, usize::MAX, trace_table)? {
                GcProgress::Complete(stats) => return Ok(stats),
                GcProgress::Active => continue,
                GcProgress::Idle => {
                    return Err(HeapError::Internal {
                        context: "local young collection made no progress",
                    }
                    .into());
                }
            }
        }
    }

    /// Return whether one local young collection is active.
    pub(crate) fn minor_gc_active(&self) -> bool {
        self.collector.minor_phase != Phase::Idle
    }

    /// Start one local young collection.
    pub(crate) fn start_young_gc(&mut self) -> HeapResult<()> {
        // reject overlapping collection work
        if self.collector.is_collecting() {
            return Err(HeapError::gc_state(HeapGcStateError::LocalGcActive));
        }

        // publish active span accounting before tracing
        self.flush_young_cursor();

        // reset minor cycle cursors
        self.clear_young_mark_bits();
        self.collector.minor_queue.clear();
        self.collector.minor_phase = Phase::Mark;
        self.collector.young_sweep_range_cursor = 0;
        self.collector.young_sweep_span_cursor = 0;
        self.collector.young_sweep_slot_cursor = 0;
        self.collector.young_dirty_extent_cursor = 0;
        self.collector.young_dirty_card_cursor = 0;
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(())
    }

    /// Drain one active local young collection at a safepoint.
    pub(crate) fn step_young_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.minor_phase == Phase::Idle {
            return Ok(GcProgress::Idle);
        }

        match self.collector.minor_phase {
            Phase::Idle => Ok(GcProgress::Idle),
            Phase::Mark => self.mark_young_gc_step(roots, usize::MAX, trace_table),
            Phase::Sweep => self.sweep_young_gc_step(roots, usize::MAX, trace_table),
        }
    }

    /// Mark reachable young blocks and remembered mature writes.
    fn mark_young_gc_step<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        self.seed_young_roots(roots)?;
        let dirty_bytes = self.scan_dirty_young_references_step(budget_bytes, trace_table)?;
        let marked_bytes = if dirty_bytes < budget_bytes {
            self.mark_young_references_step(budget_bytes - dirty_bytes, trace_table)?
        } else {
            0
        };

        // use remaining safepoint budget before returning
        if self.young_dirty_references_drained()
            && self.collector.minor_queue.is_empty()
            && dirty_bytes + marked_bytes < budget_bytes
        {
            self.collector.minor_phase = Phase::Sweep;
            let remaining_bytes = budget_bytes - dirty_bytes - marked_bytes;

            return self.sweep_young_gc_step(roots, remaining_bytes, trace_table);
        }

        // advance to sweep for the next safepoint
        if self.young_dirty_references_drained() && self.collector.minor_queue.is_empty() {
            self.collector.minor_phase = Phase::Sweep;
        }

        Ok(GcProgress::Active)
    }

    /// Seed the active young trace queue from roots and pins.
    fn seed_young_roots<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<HeapError>,
    {
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        roots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_young_reference(reference, &mut pending)
        })?;

        let pins = std::mem::take(&mut self.collector.pins);
        for reference in pins.references() {
            self.enqueue_young_reference(reference, &mut pending)?;
        }
        self.collector.pins = pins;

        self.collector.minor_queue = pending;

        Ok(())
    }

    /// Finish the active local young collection.
    fn finish_young_gc(&mut self) -> HeapResult<GcStats> {
        // recycle empty nursery metadata before publishing stats
        if self.young_usage.allocation_count() == 0 {
            self.recycle_young_space();
        }

        let stats = self.stats_after_collection(
            self.collector.young_freed_allocations,
            self.collector.young_freed_bytes,
        );
        self.gc.record_cycle(GcKind::Minor, stats);

        // reset active minor collection state
        self.collector.minor_phase = Phase::Idle;
        self.collector.minor_queue.clear();
        self.collector.young_sweep_range_cursor = 0;
        self.collector.young_sweep_span_cursor = 0;
        self.collector.young_sweep_slot_cursor = 0;
        self.collector.young_dirty_extent_cursor = 0;
        self.collector.young_dirty_card_cursor = 0;

        // keep cards that still bridge mature objects to young objects
        self.compact_dirty_extents();
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(stats)
    }

    /// Mark reachable young references within one byte budget.
    fn mark_young_references_step(
        &mut self,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        // drain bounded young trace work
        while marked_bytes < budget_bytes {
            let Some(reference) = pending.pop() else {
                break;
            };
            let Some(extent) = self.resolve_extent(reference) else {
                return Err(HeapError::invalid_heap_reference(reference));
            };

            // minor collection only traces young blocks
            if !matches!(
                extent.storage,
                HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
            ) {
                continue;
            }

            // load exact reference layout for this block
            let trace_map = self
                .trace_map_for_place(extent.storage, trace_table)
                .map_err(|error| {
                    HeapError::scan_failed(HeapOperationSource::Reference(reference), error)
                })?;

            let mut references = Vec::new();

            // enqueue every local reference discovered in this payload
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

            for reference in references {
                self.enqueue_young_reference(reference, &mut pending)?;
            }

            marked_bytes += extent.byte_len.max(1);
        }

        self.collector.minor_queue = pending;

        Ok(marked_bytes)
    }

    /// Sweep unreachable young references within one byte budget.
    fn sweep_young_gc_step<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        let mut swept_bytes = 0usize;

        self.sweep_young_ranges_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_young_spans_step(budget_bytes, &mut swept_bytes)?;

        if self.collector.young_sweep_range_cursor >= self.young.ranges.len()
            && self.collector.young_sweep_span_cursor >= self.young.spans.len()
        {
            // young relocation must drain before the mutator resumes
            self.relocate_young_survivors(roots, trace_table)?;

            return self
                .finish_young_gc()
                .map(GcProgress::Complete)
                .map_err(Into::into);
        }

        Ok(GcProgress::Active)
    }

    /// Sweep unreachable young range blocks within one byte budget.
    fn sweep_young_ranges_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.young_sweep_range_cursor < self.young.ranges.len()
        {
            let Some(block_index) = self
                .young
                .live
                .first_set_from(self.collector.young_sweep_range_cursor)
            else {
                self.collector.young_sweep_range_cursor = charge_bitmap_skip(
                    self.collector.young_sweep_range_cursor,
                    self.young.ranges.len(),
                    budget_bytes,
                    swept_bytes,
                );

                return Ok(());
            };

            self.collector.young_sweep_range_cursor = charge_bitmap_skip(
                self.collector.young_sweep_range_cursor,
                block_index,
                budget_bytes,
                swept_bytes,
            );
            if *swept_bytes >= budget_bytes {
                return Ok(());
            }
            self.collector.young_sweep_range_cursor = block_index + 1;

            let Some(block) = self.young_range(block_index) else {
                return Err(HeapError::Internal {
                    context: "live young range missing during sweep",
                });
            };
            let reference = HeapReference::new(block.first_offset);
            let byte_len = block.byte_len;

            if self.young.marked.contains(block_index) {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            self.free(reference)
                .map_err(|error| HeapError::free_failed(reference, error))?;
            self.collector.young_freed_allocations += 1;
            self.collector.young_freed_bytes += byte_len as u64;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Sweep unreachable fixed-size young spans within one byte budget.
    fn sweep_young_spans_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        self.flush_young_cursor();

        while *swept_bytes < budget_bytes
            && self.collector.young_sweep_span_cursor < self.young.spans.len()
        {
            let span_index = self.collector.young_sweep_span_cursor;
            let Some(span) = self.young.span(span_index).cloned() else {
                return Err(HeapError::internal("missing span"));
            };
            let reserved_slots = self
                .young
                .span_reserved_slot_count(span_index)
                .unwrap_or_else(|| span.slot_count());

            while *swept_bytes < budget_bytes
                && self.collector.young_sweep_slot_cursor < reserved_slots
            {
                let slot_index = self.collector.young_sweep_slot_cursor;
                self.collector.young_sweep_slot_cursor += 1;

                let Some(bits) = self.young.span_bits_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };
                if bits.freed.contains(slot_index) {
                    *swept_bytes += GC_METADATA_STEP_BYTES;

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
                self.collector.young_freed_allocations += 1;
                self.collector.young_freed_bytes += span.class.size_class as u64;
                *swept_bytes += span.class.size_class.max(1);
            }

            if self.collector.young_sweep_slot_cursor >= reserved_slots {
                self.collector.young_sweep_span_cursor += 1;
                self.collector.young_sweep_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Recycle empty young metadata without changing the reserved page span.
    fn recycle_young_space(&mut self) {
        self.young.next_offset = self.young.allocation_alignment_bytes;
        self.young.ranges.clear();
        self.young.live.clear_all();
        self.young.marked.clear_all();
        self.young.local_reference_bits.clear_all();
        self.young.shared_reference_bits.clear_all();
        self.young.spans.clear();
        self.young.span_bits.clear();
        self.young.span_cache.fill(None);
        self.young.cursor = YoungCursor::inactive();
        self.young.page_spans.fill(None);
    }

    /// Queue one young reference when it currently points into the young space.
    fn enqueue_young_reference(
        &mut self,
        reference: HeapReference,
        pending: &mut TraceQueue<HeapReference>,
    ) -> HeapResult<()> {
        // null references are not heap roots
        if reference.is_null() {
            return Ok(());
        }

        // only young references belong in the minor queue
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        if matches!(
            extent.storage,
            HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
        ) && self.mark_place(extent.storage)?
        {
            pending.push(reference);
        }

        Ok(())
    }

    /// Scan remembered mature writes within one byte budget.
    fn scan_dirty_young_references_step(
        &mut self,
        budget_bytes: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<usize> {
        let mut scanned_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        self.scan_dirty_extents_step(budget_bytes, &mut scanned_bytes, &mut pending, trace_table)?;

        self.collector.minor_queue = pending;

        Ok(scanned_bytes)
    }

    /// Scan remembered mature extent writes within one byte budget.
    fn scan_dirty_extents_step(
        &mut self,
        budget_bytes: usize,
        scanned_bytes: &mut usize,
        pending: &mut TraceQueue<HeapReference>,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        while *scanned_bytes < budget_bytes
            && self.collector.young_dirty_extent_cursor < self.collector.dirty_extents.len()
        {
            let dirty_index = self.collector.young_dirty_extent_cursor;
            let extent = self.collector.dirty_extents[dirty_index];
            let references = self.scan_next_dirty_extent_card(extent, trace_table)?;

            let Some((card_index, card_len, references)) = references else {
                self.finish_dirty_extent(extent)?;
                self.collector.young_dirty_extent_cursor += 1;
                self.collector.young_dirty_card_cursor = 0;
                *scanned_bytes += 1;

                continue;
            };

            let has_young_reference = self.contains_young_reference(&references)?;
            for reference in references {
                self.enqueue_young_reference(reference, pending)?;
            }

            self.finish_dirty_extent_card(extent, card_index, has_young_reference)?;
            *scanned_bytes += card_len.max(1);
        }

        Ok(())
    }

    /// Scan one dirty card from one mature extent.
    fn scan_next_dirty_extent_card(
        &self,
        extent: DirtyExtent,
        trace_table: &TraceTable,
    ) -> HeapResult<Option<(usize, usize, Vec<HeapReference>)>> {
        match extent {
            DirtyExtent::Span(span_index) => {
                self.scan_next_dirty_span_card(span_index, trace_table)
            }
            DirtyExtent::Large(block_id) => self.scan_next_dirty_large_card(block_id),
        }
    }

    /// Scan one dirty card from one mature span.
    fn scan_next_dirty_span_card(
        &self,
        span_index: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<Option<(usize, usize, Vec<HeapReference>)>> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::scan_failed(
                HeapOperationSource::Span(span_index),
                HeapError::internal("missing span"),
            ));
        };
        let card_cursor = self.collector.young_dirty_card_cursor;
        let Some((card_index, card_start, card_len)) =
            span.dirty_cards.next_dirty_card_from(card_cursor)
        else {
            return Ok(None);
        };
        let card_end = card_start + card_len;
        let first_slot = card_start / span.class.size_class;
        let last_slot = (card_end - 1) / span.class.size_class;
        let end_slot = (last_slot + 1).min(span.slot_count);
        let mut references = Vec::new();

        for slot_index in first_slot..end_slot {
            if !span.occupied.contains(slot_index) {
                continue;
            }

            let trace_map = self.small_slot_trace_map(span_index, slot_index, trace_table)?;
            if !trace_map.has_local_reference() {
                continue;
            }

            let slot_start = span.class.size_class * slot_index;
            let slot_end = slot_start + span.class.size_class;
            let overlap_start = card_start.max(slot_start);
            let overlap_end = card_end.min(slot_end);

            if overlap_start >= overlap_end {
                continue;
            }

            let local_start = overlap_start - slot_start;
            let local_len = overlap_end - overlap_start;
            let base_address = self.mapping.base_address() + span.first_offset + slot_start;

            scan_references::<HeapReference>(
                &trace_map,
                ReferenceInput::mapped(base_address),
                ReferenceRange::bytes(local_start, local_len),
                &mut references,
            )
            .map_err(|error| {
                HeapError::scan_failed(HeapOperationSource::Span(span_index), error)
            })?;
        }

        Ok(Some((card_index, card_len, references)))
    }

    /// Finish one scanned dirty extent card.
    fn finish_dirty_extent_card(
        &mut self,
        extent: DirtyExtent,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        match extent {
            DirtyExtent::Span(span_index) => {
                self.finish_dirty_span_card(span_index, card_index, has_young_reference)
            }
            DirtyExtent::Large(block_id) => {
                self.finish_dirty_large_card(block_id, card_index, has_young_reference)
            }
        }
    }

    /// Finish one scanned dirty span card.
    fn finish_dirty_span_card(
        &mut self,
        span_index: usize,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        if !has_young_reference {
            span.dirty_cards.clear_card(card_index);
        }
        let is_empty = span.dirty_cards.is_empty();
        if is_empty {
            span.is_dirty_queued = false;
        }

        self.collector.young_dirty_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_extent_cursor += 1;
            self.collector.young_dirty_card_cursor = 0;
        }

        Ok(())
    }

    /// Scan one dirty card from one mature large block.
    fn scan_next_dirty_large_card(
        &self,
        block_id: LargeBlockId,
    ) -> HeapResult<Option<(usize, usize, Vec<HeapReference>)>> {
        let Some(block) = self.large_block(block_id) else {
            return Err(HeapError::scan_failed(
                HeapOperationSource::LargeBlock(block_id.id()),
                HeapError::internal("missing large block"),
            ));
        };
        let card_cursor = self.collector.young_dirty_card_cursor;
        let Some((card_index, card_start, card_len)) =
            block.dirty_cards.next_dirty_card_from(card_cursor)
        else {
            return Ok(None);
        };
        let mut references = Vec::new();

        let base_address = self.mapping.base_address() + block.first_offset;
        scan_references::<HeapReference>(
            &block.trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(card_start, card_len),
            &mut references,
        )
        .map_err(|error| {
            HeapError::scan_failed(HeapOperationSource::LargeBlock(block_id.id()), error)
        })?;

        Ok(Some((card_index, card_len, references)))
    }

    /// Finish one scanned dirty large-block card.
    fn finish_dirty_large_card(
        &mut self,
        block_id: LargeBlockId,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(block) = self.large_block_mut(block_id) else {
            return Err(HeapError::internal("missing large block"));
        };

        if !has_young_reference {
            block.dirty_cards.clear_card(card_index);
        }
        let is_empty = block.dirty_cards.is_empty();
        if is_empty {
            block.is_dirty_queued = false;
        }

        self.collector.young_dirty_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_extent_cursor += 1;
            self.collector.young_dirty_card_cursor = 0;
        }

        Ok(())
    }

    /// Finish one queued dirty extent with no remaining dirty cards.
    fn finish_dirty_extent(&mut self, extent: DirtyExtent) -> HeapResult<()> {
        match extent {
            DirtyExtent::Span(span_index) => {
                let Some(span) = self.span_mut(span_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.is_dirty_queued = !span.dirty_cards.is_empty();
            }
            DirtyExtent::Large(block_id) => {
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.is_dirty_queued = !block.dirty_cards.is_empty();
            }
        }

        Ok(())
    }

    /// Return whether any reference currently points into young space.
    fn contains_young_reference(&self, references: &[HeapReference]) -> HeapResult<bool> {
        for reference in references {
            if reference.is_null() {
                continue;
            }

            let Some(extent) = self.resolve_extent(*reference) else {
                return Err(HeapError::invalid_heap_reference(*reference));
            };

            if matches!(
                extent.storage,
                HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_)
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Keep only mature remembered-set entries that still have dirty cards.
    fn compact_dirty_extents(&mut self) {
        self.collector.dirty_extents.retain(|extent| match extent {
            DirtyExtent::Span(span_index) => self
                .small
                .spans
                .get(*span_index)
                .is_some_and(|span| span.is_dirty_queued),
            DirtyExtent::Large(block_id) => block_id
                .index()
                .ok()
                .and_then(|index| self.large.blocks.get(index))
                .is_some_and(|block| block.is_dirty_queued),
        });
    }

    /// Return whether remembered young roots are fully scanned.
    fn young_dirty_references_drained(&self) -> bool {
        self.collector.young_dirty_extent_cursor >= self.collector.dirty_extents.len()
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
    let skipped_words = skipped_bits.div_ceil(GC_METADATA_WORD_BITS);
    let remaining_budget = budget_bytes - *swept_bytes;
    if skipped_words <= remaining_budget {
        *swept_bytes += skipped_words;

        return end;
    }

    *swept_bytes = budget_bytes;

    start + remaining_budget * GC_METADATA_WORD_BITS
}
