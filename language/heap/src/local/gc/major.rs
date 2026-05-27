use crate::local::space::{
    GcKind, GcStats, HeapLocation, HeapPlace, HeapSpace, LocalGcPhase, LocalTraceWork,
    MajorSweepCursor, YoungPlace,
};
use crate::{
    GcProgress, HeapError, HeapReference, HeapResult, RootSlot, ScanSource, scan_heap_references,
    scan_heap_references_in_range,
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

    /// Publish one allocation to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == LocalGcPhase::Idle {
            return Ok(());
        }

        self.mark_place(place)?;

        self.enqueue_location_heap_references(reference, place, 0, usize::MAX)
    }

    /// Queue local references written into one active local major cycle.
    pub(crate) fn write_major_barrier(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // inactive collector
        if self.collector.major_phase == LocalGcPhase::Idle {
            return Ok(());
        }

        self.enqueue_location_heap_references(location.base, location.place, byte_offset, byte_len)
    }

    /// Queue local references from one heap payload range into the active major cycle.
    fn enqueue_location_heap_references(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // skip noscan payloads
        let trace_map = self.trace_map_for_place(place)?;
        if !trace_map.has_local_reference() {
            return Ok(());
        }

        // resolve the scan window inside the allocation payload
        let location = HeapLocation {
            place,
            base: reference,
            byte_offset: 0,
            byte_len: self.byte_len_for_place(place)?,
        };
        let scan_len = byte_len.min(location.byte_len - byte_offset);
        let mut references = Vec::new();

        // scan local reference slots in mapped heap memory
        let base_address = self.mapping.base_address() + location.base.offset();
        let result = scan_heap_references_in_range(
            &trace_map,
            byte_offset,
            scan_len,
            base_address,
            &mut references,
        );

        if let Err(error) = result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
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
    ) -> Result<GcStats, E>
    where
        E: From<HeapError>,
    {
        // collect the nursery first so full collection sees mature references
        let _minor = self.collect_minor(roots)?;

        self.start_major_gc(roots)?;

        // drain the active major cycle synchronously
        loop {
            match self.step_major_gc(roots, usize::MAX)? {
                GcProgress::Complete(stats) => return Ok(stats),
                GcProgress::Active => continue,
                GcProgress::Idle => {
                    return Err(HeapError::InvariantViolation {
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
            return Err(HeapError::HeapCollectionActive.into());
        }

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
                let marked_bytes = self.mark_reachable_references_step(budget_bytes)?;

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
                let marked_bytes = self.mark_reachable_references_step(budget_bytes)?;
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
        self.young.flush_run_cursor();
        self.collector.major_sweep = MajorSweepCursor {
            small_span_limit: self.small.spans.len(),
            large_limit: self.large.allocations.len(),
            ..MajorSweepCursor::default()
        };
    }

    /// Sweep unreachable references within one byte budget.
    fn sweep_unreachable_references_step(&mut self, budget_bytes: usize) -> HeapResult<GcProgress> {
        let mut swept_bytes = 0usize;

        self.sweep_major_young_ranges_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_young_runs_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_small_spans_step(budget_bytes, &mut swept_bytes)?;
        self.sweep_major_large_allocations_step(budget_bytes, &mut swept_bytes)?;

        if self.major_sweep_drained() {
            return self.finish_major_gc().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Sweep unreachable young range allocations within one byte budget.
    fn sweep_major_young_ranges_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_range_cursor < self.young.ranges.len()
        {
            let Some(allocation_index) = self
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
                allocation_index,
                budget_bytes,
                swept_bytes,
            );
            if *swept_bytes >= budget_bytes {
                return Ok(());
            }
            self.collector.major_sweep.young_range_cursor = allocation_index + 1;

            let Some(allocation) = self.young_range(allocation_index) else {
                return Err(HeapError::InvariantViolation {
                    context: "live young range missing during major sweep",
                });
            };
            let reference = HeapReference::new(allocation.first_offset);
            let byte_len = allocation.byte_len;

            if self.young.marked.contains(allocation_index) {
                *swept_bytes += byte_len.max(1);

                continue;
            }

            self.free_swept_reference(reference, byte_len)?;
            *swept_bytes += byte_len.max(1);
        }

        Ok(())
    }

    /// Sweep unreachable young run slots within one byte budget.
    fn sweep_major_young_runs_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.young_run_cursor < self.young.runs.len()
        {
            let run_index = self.collector.major_sweep.young_run_cursor;
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
                && self.collector.major_sweep.young_slot_cursor < reserved_slots
            {
                let slot_index = self.collector.major_sweep.young_slot_cursor;
                self.collector.major_sweep.young_slot_cursor += 1;

                let Some(bits) = self.young.run_bits_mut(run_index) else {
                    return Err(HeapError::MissingSpan {
                        span_index: run_index,
                    });
                };
                if bits.freed.contains(slot_index) {
                    *swept_bytes += METADATA_STEP_BYTES;

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
                self.collector.major_freed_allocations += 1;
                self.collector.major_freed_bytes += run.size_class as u64;
                *swept_bytes += run.size_class.max(1);
            }

            if self.collector.major_sweep.young_slot_cursor >= reserved_slots {
                self.collector.major_sweep.young_run_cursor += 1;
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
                return Err(HeapError::MissingSpan { span_index });
            };
            let slot_count = span.slot_count;
            let size_class = span.class.size_class;

            while *swept_bytes < budget_bytes
                && self.collector.major_sweep.small_slot_cursor < slot_count
            {
                let slot_index = self.collector.major_sweep.small_slot_cursor;
                self.collector.major_sweep.small_slot_cursor += 1;

                let Some(span) = self.span(span_index) else {
                    return Err(HeapError::MissingSpan { span_index });
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

    /// Sweep unreachable large allocations within one byte budget.
    fn sweep_major_large_allocations_step(
        &mut self,
        budget_bytes: usize,
        swept_bytes: &mut usize,
    ) -> HeapResult<()> {
        while *swept_bytes < budget_bytes
            && self.collector.major_sweep.large_cursor < self.collector.major_sweep.large_limit
        {
            let allocation_index = self.collector.major_sweep.large_cursor;
            self.collector.major_sweep.large_cursor += 1;

            let Some(allocation) = self.large.allocations.get(allocation_index) else {
                return Err(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_index as u64 + 1,
                });
            };
            if !allocation.is_live {
                *swept_bytes += METADATA_STEP_BYTES;

                continue;
            }

            let reference = HeapReference::new(allocation.first_offset);
            let byte_len = allocation.byte_len;
            if allocation.mark_epoch == self.collector.mark_epoch {
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
            && self.collector.major_sweep.young_run_cursor >= self.young.runs.len()
            && self.collector.major_sweep.small_span_cursor
                >= self.collector.major_sweep.small_span_limit
            && self.collector.major_sweep.large_cursor >= self.collector.major_sweep.large_limit
    }

    /// Free one unreachable allocation discovered by major sweep.
    fn free_swept_reference(
        &mut self,
        reference: HeapReference,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.free(reference)
            .map_err(|error| HeapError::HeapFreeFailed {
                reference,
                error: Box::new(error),
            })?;
        self.collector.major_freed_allocations += 1;
        self.collector.major_freed_bytes += byte_len as u64;

        Ok(())
    }

    /// Mark reachable heap references within one byte budget.
    fn mark_reachable_references_step(&mut self, budget_bytes: usize) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;

        // trace bounded mark work
        while marked_bytes < budget_bytes {
            // claim the next bounded mark item
            let Some(work) = self.collector.major_queue.pop() else {
                break;
            };

            match work {
                // large allocations are scanned page by page
                LocalTraceWork::LargeRange { reference, start } => {
                    marked_bytes += self.trace_large_range(reference, start)?;

                    continue;
                }

                // small and young allocations are scanned as one mark item
                LocalTraceWork::Reference(reference) => {
                    let Some(location) = self.resolve_location(reference) else {
                        return Err(HeapError::InvalidHeapReference { reference });
                    };

                    marked_bytes += location.byte_len.max(1);

                    // read layout side metadata for this payload
                    let trace_map = self.trace_map_for_place(location.place).map_err(|error| {
                        HeapError::HeapScanFailed {
                            source: ScanSource::Reference(reference),
                            error: Box::new(error),
                        }
                    })?;

                    let mut references = Vec::new();

                    // scan every local reference discovered in this payload
                    let base_address = self.mapping.base_address() + location.base.offset();
                    let trace_result =
                        scan_heap_references(&trace_map, base_address, &mut references);

                    if let Err(error) = trace_result {
                        return Err(HeapError::HeapScanFailed {
                            source: ScanSource::Reference(reference),
                            error: Box::new(error),
                        });
                    }

                    // validate and enqueue after the read borrow has ended
                    for reference in references {
                        if !reference.is_null() && self.resolve_location(reference).is_none() {
                            return Err(HeapError::InvalidHeapReference { reference });
                        }

                        self.enqueue_major_reference(reference)?;
                    }
                }
            }
        }

        Ok(marked_bytes)
    }

    /// Trace one page-sized range from one local large allocation.
    fn trace_large_range(&mut self, reference: HeapReference, start: usize) -> HeapResult<usize> {
        // resolve and verify the large allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let HeapPlace::Large(_) = location.place else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // skip empty ranges and noscan payloads
        let trace_map = self.trace_map_for_place(location.place).map_err(|error| {
            HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            }
        })?;
        if !trace_map.has_local_reference() || start >= location.byte_len {
            return Ok(0);
        }

        // scan at most one allocator page
        let range_len = self.allocator().page_bytes().min(location.byte_len - start);
        let mut references = Vec::new();
        let base_address = self.mapping.base_address() + location.base.offset();
        let trace_result = scan_heap_references_in_range(
            &trace_map,
            start,
            range_len,
            base_address,
            &mut references,
        );

        if let Err(error) = trace_result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        // enqueue discovered local references after the read borrow ends
        for reference in references {
            if !reference.is_null() && self.resolve_location(reference).is_none() {
                return Err(HeapError::InvalidHeapReference { reference });
            }

            self.enqueue_major_reference(reference)?;
        }

        // continue this large allocation on a later step
        let next_start = start + range_len;
        if next_start < location.byte_len {
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

        // mark the referenced allocation before queueing scan work
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        if self.mark_place(location.place)? {
            // marked noscan allocations need no queued scan work
            let trace_map = self.trace_map_for_place(location.place)?;
            if !trace_map.has_local_reference() {
                return Ok(());
            }

            // large allocations are sliced to keep major steps bounded
            if matches!(location.place, HeapPlace::Large(_)) {
                self.collector.major_queue.push(LocalTraceWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller allocations are one mark item
            self.collector
                .major_queue
                .push(LocalTraceWork::Reference(reference));
        }

        Ok(())
    }

    /// Clear every young-space mark bit.
    pub(super) fn clear_young_mark_bits(&mut self) {
        self.young.marked.clear_all();
        for bits in &mut self.young.run_bits {
            bits.marked.clear_all();
        }
    }

    /// Return whether one heap place is marked in the active cycle.
    pub(super) fn is_marked_place(&self, place: HeapPlace) -> HeapResult<bool> {
        // dispatch by physical heap place
        match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((allocation_index, _allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                Ok(self.young.marked.contains(allocation_index))
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(bits) = self.young.run_bits(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                Ok(bits.marked.contains(slot.slot_index()))
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                Ok(span.mark_epoch == self.collector.mark_epoch
                    && span.marked.contains(slot.slot_index()))
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                Ok(allocation.mark_epoch == self.collector.mark_epoch)
            }
        }
    }

    /// Mark one heap place and return whether this was the first mark.
    pub(super) fn mark_place(&mut self, place: HeapPlace) -> HeapResult<bool> {
        // skip already marked places
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        // mark by physical heap place
        match place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((allocation_index, _allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                self.young.marked.set(allocation_index);
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(bits) = self.young.run_bits_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                bits.marked.set(slot.slot_index());
            }
            HeapPlace::Small(slot) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                span.ensure_mark_epoch(mark_epoch);
                span.marked.set(slot.slot_index());
            }
            HeapPlace::Large(allocation_id) => {
                let mark_epoch = self.collector.mark_epoch;
                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                allocation.mark_epoch = mark_epoch;
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
