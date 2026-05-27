use crate::local::space::{
    GC_METADATA_STEP_BYTES, GC_METADATA_WORD_BITS, GcKind, GcStats, HeapPlace, HeapSpace,
    HeapTraceQueue, LargeAllocationId, YoungGcPhase, YoungPlace, YoungRunCursor,
};
use crate::{
    GcProgress, HeapError, HeapReference, HeapResult, RootSlot, ScanSource, scan_heap_references,
    scan_heap_references_in_range, slot_trace_map,
};

impl HeapSpace {
    /// Perform one young-generation collection over mutable heap roots.
    pub fn collect_minor<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        self.start_young_gc()?;

        loop {
            match self.step_young_gc(roots, usize::MAX)? {
                GcProgress::Complete(stats) => return Ok(stats),
                GcProgress::Active => continue,
                GcProgress::Idle => {
                    return Err(HeapError::InvariantViolation {
                        context: "local young collection made no progress",
                    }
                    .into());
                }
            }
        }
    }

    /// Return whether one local young collection is active.
    pub(crate) fn young_gc_active(&self) -> bool {
        self.collector.young_phase != YoungGcPhase::Idle
    }

    /// Start one local young collection.
    pub(crate) fn start_young_gc(&mut self) -> HeapResult<()> {
        // reject overlapping collection work
        if self.collector.is_collecting() {
            return Err(HeapError::HeapCollectionActive);
        }

        // reset minor cycle cursors
        self.clear_young_mark_bits();
        self.collector.minor_queue.clear();
        self.collector.young_phase = YoungGcPhase::Mark;
        self.collector.young_sweep_range_cursor = 0;
        self.collector.young_sweep_run_cursor = 0;
        self.collector.young_sweep_slot_cursor = 0;
        self.collector.young_dirty_span_cursor = 0;
        self.collector.young_dirty_span_card_cursor = 0;
        self.collector.young_dirty_large_cursor = 0;
        self.collector.young_dirty_large_card_cursor = 0;
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(())
    }

    /// Drain one active local young collection at a safepoint.
    pub(crate) fn step_young_gc<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        // no active work
        if budget_bytes == 0 || self.collector.young_phase == YoungGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        match self.collector.young_phase {
            YoungGcPhase::Idle => Ok(GcProgress::Idle),
            YoungGcPhase::Mark => self.mark_young_gc_step(roots, usize::MAX),
            YoungGcPhase::Sweep => self.sweep_young_gc_step(roots, usize::MAX),
        }
    }

    /// Mark reachable young allocations and remembered mature writes.
    fn mark_young_gc_step<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        self.seed_young_roots(roots)?;
        let dirty_bytes = self.scan_dirty_young_references_step(budget_bytes)?;
        let marked_bytes = if dirty_bytes < budget_bytes {
            self.mark_young_references_step(budget_bytes - dirty_bytes)?
        } else {
            0
        };

        // use remaining safepoint budget before returning
        if self.young_dirty_references_drained()
            && self.collector.minor_queue.is_empty()
            && dirty_bytes + marked_bytes < budget_bytes
        {
            self.collector.young_phase = YoungGcPhase::Sweep;
            let remaining_bytes = budget_bytes - dirty_bytes - marked_bytes;

            return self.sweep_young_gc_step(roots, remaining_bytes);
        }

        // advance to sweep for the next safepoint
        if self.young_dirty_references_drained() && self.collector.minor_queue.is_empty() {
            self.collector.young_phase = YoungGcPhase::Sweep;
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
        self.collector.young_phase = YoungGcPhase::Idle;
        self.collector.minor_queue.clear();
        self.collector.young_sweep_range_cursor = 0;
        self.collector.young_sweep_run_cursor = 0;
        self.collector.young_sweep_slot_cursor = 0;
        self.collector.young_dirty_span_cursor = 0;
        self.collector.young_dirty_span_card_cursor = 0;
        self.collector.young_dirty_large_cursor = 0;
        self.collector.young_dirty_large_card_cursor = 0;

        // keep cards that still bridge mature objects to young objects
        self.compact_dirty_young_reference_queues();
        self.collector.young_freed_allocations = 0;
        self.collector.young_freed_bytes = 0;

        Ok(stats)
    }

    /// Mark reachable young references within one byte budget.
    fn mark_young_references_step(&mut self, budget_bytes: usize) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        // drain bounded young trace work
        while marked_bytes < budget_bytes {
            let Some(reference) = pending.pop() else {
                break;
            };
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };

            // minor collection only traces young allocations
            if !matches!(
                location.place,
                HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
            ) {
                continue;
            }

            // load exact reference layout for this allocation
            let trace_map = self.trace_map_for_place(location.place).map_err(|error| {
                HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                }
            })?;

            let mut references = Vec::new();

            // enqueue every local reference discovered in this payload
            let base_address = self.mapping.base_address() + location.base.offset();
            let trace_result = scan_heap_references(&trace_map, base_address, &mut references);

            if let Err(error) = trace_result {
                return Err(HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                });
            }

            for reference in references {
                self.enqueue_young_reference(reference, &mut pending)?;
            }

            marked_bytes += location.byte_len.max(1);
        }

        self.collector.minor_queue = pending;

        Ok(marked_bytes)
    }

    /// Sweep unreachable young references within one byte budget.
    fn sweep_young_gc_step<E>(
        &mut self,
        roots: &mut impl FnMut(&mut dyn FnMut(RootSlot<'_>) -> HeapResult<()>) -> Result<(), E>,
        budget_bytes: usize,
    ) -> Result<GcProgress, E>
    where
        E: From<HeapError>,
    {
        let mut swept_bytes = 0usize;

        self.sweep_young_ranges_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_young_runs_step(budget_bytes, &mut swept_bytes)?;

        if self.collector.young_sweep_range_cursor >= self.young.ranges.len()
            && self.collector.young_sweep_run_cursor >= self.young.runs.len()
        {
            // young relocation must drain before the mutator resumes
            self.relocate_young_survivors(roots)?;

            return self
                .finish_young_gc()
                .map(GcProgress::Complete)
                .map_err(Into::into);
        }

        Ok(GcProgress::Active)
    }

    /// Sweep unreachable young range allocations within one byte budget.
    fn sweep_young_ranges_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.young_sweep_range_cursor < self.young.ranges.len()
        {
            let Some(allocation_index) = self
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
                allocation_index,
                budget_bytes,
                swept_bytes,
            );
            if *swept_bytes >= budget_bytes {
                return Ok(());
            }
            self.collector.young_sweep_range_cursor = allocation_index + 1;

            let Some(allocation) = self.young_range(allocation_index) else {
                return Err(HeapError::InvariantViolation {
                    context: "live young range missing during sweep",
                });
            };
            let reference = HeapReference::new(allocation.first_offset);
            let byte_len = allocation.byte_len;

            if self.young.marked.contains(allocation_index) {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            self.free(reference)
                .map_err(|error| HeapError::HeapFreeFailed {
                    reference,
                    error: Box::new(error),
                })?;
            self.collector.young_freed_allocations += 1;
            self.collector.young_freed_bytes += byte_len as u64;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Sweep unreachable fixed-size young runs within one byte budget.
    fn sweep_young_runs_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        self.young.flush_run_cursor();

        while *swept_bytes < budget_bytes
            && self.collector.young_sweep_run_cursor < self.young.runs.len()
        {
            let run_index = self.collector.young_sweep_run_cursor;
            let Some(run) = self.young.run(run_index).cloned() else {
                return Err(HeapError::MissingSpan {
                    span_index: run_index,
                });
            };
            let reserved_slots = self
                .young
                .run_reserved_slot_count(run_index)
                .unwrap_or_else(|| run.slot_count());

            while *swept_bytes < budget_bytes
                && self.collector.young_sweep_slot_cursor < reserved_slots
            {
                let slot_index = self.collector.young_sweep_slot_cursor;
                self.collector.young_sweep_slot_cursor += 1;

                let Some(bits) = self.young.run_bits_mut(run_index) else {
                    return Err(HeapError::MissingSpan {
                        span_index: run_index,
                    });
                };
                if bits.freed.contains(slot_index) {
                    *swept_bytes += GC_METADATA_STEP_BYTES;

                    continue;
                }

                if bits.marked.contains(slot_index) {
                    *swept_bytes += run.size_class.max(1);

                    continue;
                }

                bits.freed.set(slot_index);
                bits.marked.clear(slot_index);
                let reference = HeapReference::new(run.slot_offset(slot_index));
                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(run.size_class);
                self.collector.young_freed_allocations += 1;
                self.collector.young_freed_bytes += run.size_class as u64;
                *swept_bytes += run.size_class.max(1);
            }

            if self.collector.young_sweep_slot_cursor >= reserved_slots {
                self.collector.young_sweep_run_cursor += 1;
                self.collector.young_sweep_slot_cursor = 0;
            }
        }

        Ok(())
    }

    /// Recycle empty young metadata without changing the reserved page run.
    fn recycle_young_space(&mut self) {
        self.young.next_offset = self.young.allocation_alignment_bytes;
        self.young.ranges.clear();
        self.young.live.clear_all();
        self.young.marked.clear_all();
        self.young.local_reference_bits.clear_all();
        self.young.shared_reference_bits.clear_all();
        self.young.runs.clear();
        self.young.run_bits.clear();
        self.young.run_buckets.fill(None);
        self.young.run_cursor = YoungRunCursor::inactive();
        self.young.page_runs.fill(None);
    }

    /// Queue one young reference when it currently points into the young space.
    fn enqueue_young_reference(
        &mut self,
        reference: HeapReference,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        // null references are not heap roots
        if reference.is_null() {
            return Ok(());
        }

        // only young references belong in the minor queue
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        if matches!(
            location.place,
            HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
        ) && self.mark_place(location.place)?
        {
            pending.push(reference);
        }

        Ok(())
    }

    /// Scan remembered mature writes within one byte budget.
    fn scan_dirty_young_references_step(&mut self, budget_bytes: usize) -> HeapResult<usize> {
        let mut scanned_bytes = 0usize;
        let mut pending = std::mem::take(&mut self.collector.minor_queue);

        self.scan_dirty_spans_step(budget_bytes, &mut scanned_bytes, &mut pending)?;
        self.scan_dirty_large_allocations_step(budget_bytes, &mut scanned_bytes, &mut pending)?;

        self.collector.minor_queue = pending;

        Ok(scanned_bytes)
    }

    /// Scan remembered mature span writes within one byte budget.
    fn scan_dirty_spans_step(
        &mut self,
        budget_bytes: usize,
        scanned_bytes: &mut usize,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        while *scanned_bytes < budget_bytes
            && self.collector.young_dirty_span_cursor < self.collector.dirty_spans.len()
        {
            let dirty_index = self.collector.young_dirty_span_cursor;
            let span_index = self.collector.dirty_spans[dirty_index];
            let references = self.scan_next_dirty_span_card(span_index)?;

            let Some((card_index, card_len, references)) = references else {
                self.finish_dirty_span(span_index)?;
                self.collector.young_dirty_span_cursor += 1;
                self.collector.young_dirty_span_card_cursor = 0;
                *scanned_bytes += 1;

                continue;
            };

            let has_young_reference = self.contains_young_reference(&references)?;
            for reference in references {
                self.enqueue_young_reference(reference, pending)?;
            }

            self.finish_dirty_span_card(span_index, card_index, has_young_reference)?;
            *scanned_bytes += card_len.max(1);
        }

        Ok(())
    }

    /// Scan one dirty card from one mature span.
    fn scan_next_dirty_span_card(
        &self,
        span_index: usize,
    ) -> HeapResult<Option<(usize, usize, Vec<HeapReference>)>> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Span(span_index),
                error: Box::new(HeapError::MissingSpan { span_index }),
            });
        };
        let card_cursor = self.collector.young_dirty_span_card_cursor;
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

            let trace_map = slot_trace_map(
                &span.local_reference_bits,
                &span.shared_reference_bits,
                slot_index,
                span.class.size_class,
                span.class.size_class,
            );
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

            scan_heap_references_in_range(
                &trace_map,
                local_start,
                local_len,
                base_address,
                &mut references,
            )
            .map_err(|error| HeapError::HeapScanFailed {
                source: ScanSource::Span(span_index),
                error: Box::new(error),
            })?;
        }

        Ok(Some((card_index, card_len, references)))
    }

    /// Finish one scanned dirty span card.
    fn finish_dirty_span_card(
        &mut self,
        span_index: usize,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };

        if !has_young_reference {
            span.dirty_cards.clear_card(card_index);
        }
        let is_empty = span.dirty_cards.is_empty();
        if is_empty {
            span.is_dirty_queued = false;
        }

        self.collector.young_dirty_span_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_span_cursor += 1;
            self.collector.young_dirty_span_card_cursor = 0;
        }

        Ok(())
    }

    /// Finish one queued dirty span with no remaining dirty cards.
    fn finish_dirty_span(&mut self, span_index: usize) -> HeapResult<()> {
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };

        span.is_dirty_queued = !span.dirty_cards.is_empty();

        Ok(())
    }

    /// Scan remembered mature large-allocation writes within one byte budget.
    fn scan_dirty_large_allocations_step(
        &mut self,
        budget_bytes: usize,
        scanned_bytes: &mut usize,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        while *scanned_bytes < budget_bytes
            && self.collector.young_dirty_large_cursor
                < self.collector.dirty_large_allocations.len()
        {
            let dirty_index = self.collector.young_dirty_large_cursor;
            let allocation_id = self.collector.dirty_large_allocations[dirty_index];
            let references = self.scan_next_dirty_large_card(allocation_id)?;

            let Some((card_index, card_len, references)) = references else {
                self.finish_dirty_large_allocation(allocation_id)?;
                self.collector.young_dirty_large_cursor += 1;
                self.collector.young_dirty_large_card_cursor = 0;
                *scanned_bytes += 1;

                continue;
            };

            let has_young_reference = self.contains_young_reference(&references)?;
            for reference in references {
                self.enqueue_young_reference(reference, pending)?;
            }

            self.finish_dirty_large_card(allocation_id, card_index, has_young_reference)?;
            *scanned_bytes += card_len.max(1);
        }

        Ok(())
    }

    /// Scan one dirty card from one mature large allocation.
    fn scan_next_dirty_large_card(
        &self,
        allocation_id: LargeAllocationId,
    ) -> HeapResult<Option<(usize, usize, Vec<HeapReference>)>> {
        let Some(allocation) = self.large_allocation(allocation_id) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::LargeAllocation(allocation_id.id()),
                error: Box::new(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                }),
            });
        };
        let card_cursor = self.collector.young_dirty_large_card_cursor;
        let Some((card_index, card_start, card_len)) =
            allocation.dirty_cards.next_dirty_card_from(card_cursor)
        else {
            return Ok(None);
        };
        let mut references = Vec::new();

        let base_address = self.mapping.base_address() + allocation.first_offset;
        scan_heap_references_in_range(
            &allocation.trace_map,
            card_start,
            card_len,
            base_address,
            &mut references,
        )
        .map_err(|error| HeapError::HeapScanFailed {
            source: ScanSource::LargeAllocation(allocation_id.id()),
            error: Box::new(error),
        })?;

        Ok(Some((card_index, card_len, references)))
    }

    /// Finish one scanned dirty large-allocation card.
    fn finish_dirty_large_card(
        &mut self,
        allocation_id: LargeAllocationId,
        card_index: usize,
        has_young_reference: bool,
    ) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation_mut(allocation_id) else {
            return Err(HeapError::MissingLargeAllocation {
                allocation_id: allocation_id.id(),
            });
        };

        if !has_young_reference {
            allocation.dirty_cards.clear_card(card_index);
        }
        let is_empty = allocation.dirty_cards.is_empty();
        if is_empty {
            allocation.is_dirty_queued = false;
        }

        self.collector.young_dirty_large_card_cursor = card_index + 1;
        if is_empty {
            self.collector.young_dirty_large_cursor += 1;
            self.collector.young_dirty_large_card_cursor = 0;
        }

        Ok(())
    }

    /// Finish one queued dirty large allocation with no remaining dirty cards.
    fn finish_dirty_large_allocation(
        &mut self,
        allocation_id: LargeAllocationId,
    ) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation_mut(allocation_id) else {
            return Err(HeapError::MissingLargeAllocation {
                allocation_id: allocation_id.id(),
            });
        };

        allocation.is_dirty_queued = !allocation.dirty_cards.is_empty();

        Ok(())
    }

    /// Return whether any reference currently points into young space.
    fn contains_young_reference(&self, references: &[HeapReference]) -> HeapResult<bool> {
        for reference in references {
            if reference.is_null() {
                continue;
            }

            let Some(location) = self.resolve_location(*reference) else {
                return Err(HeapError::InvalidHeapReference {
                    reference: *reference,
                });
            };

            if matches!(
                location.place,
                HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
            ) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Keep only mature remembered-set entries that still have dirty cards.
    fn compact_dirty_young_reference_queues(&mut self) {
        self.collector.dirty_spans.retain(|span_index| {
            self.small
                .spans
                .get(*span_index)
                .is_some_and(|span| span.is_dirty_queued)
        });
        self.collector
            .dirty_large_allocations
            .retain(|allocation_id| {
                allocation_id
                    .index()
                    .ok()
                    .and_then(|index| self.large.allocations.get(index))
                    .is_some_and(|allocation| allocation.is_dirty_queued)
            });
    }

    /// Return whether remembered young roots are fully scanned.
    fn young_dirty_references_drained(&self) -> bool {
        self.collector.young_dirty_span_cursor >= self.collector.dirty_spans.len()
            && self.collector.young_dirty_large_cursor
                >= self.collector.dirty_large_allocations.len()
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
