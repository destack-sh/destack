use crate::local::space::{
    GcKind, GcStats, HeapLocation, HeapPlace, HeapSpace, LargeAllocationId, LocalGcPhase,
    LocalTraceWork, YoungPlace,
};
use crate::{
    GcProgress, HeapError, HeapReference, HeapResult, RootSet, RootSlot, ScanSource,
    scan_heap_references, scan_heap_references_in_range,
};

impl HeapSpace {
    /// Return whether one local major collection is active.
    pub(crate) fn major_gc_active(&self) -> bool {
        self.major_phase != LocalGcPhase::Idle
    }

    /// Publish one allocation to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
    ) -> HeapResult<()> {
        // inactive collector
        if self.major_phase != LocalGcPhase::Mark {
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
        if self.major_phase != LocalGcPhase::Mark {
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
    pub fn collect_full<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSet,
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
    pub(crate) fn start_major_gc<R>(&mut self, roots: &mut R) -> Result<(), R::Error>
    where
        R: RootSet,
    {
        // reject overlapping collection work
        if self.is_collecting {
            return Err(HeapError::HeapCollectionActive.into());
        }

        // reset cycle state
        self.clear_mark_bits()?;
        self.major_trace_queue.clear();
        self.major_sweep_references.clear();
        self.major_sweep_cursor = 0;
        self.major_freed_allocations = 0;
        self.major_freed_bytes = 0;
        self.major_phase = LocalGcPhase::Mark;
        self.is_collecting = true;

        // seed initial roots
        self.seed_major_roots(roots)?;

        Ok(())
    }

    /// Perform bounded work for one active local major collection.
    pub(crate) fn step_major_gc<R>(
        &mut self,
        roots: &mut R,
        budget_bytes: usize,
    ) -> Result<GcProgress, R::Error>
    where
        R: RootSet,
    {
        // no active work
        if budget_bytes == 0 || self.major_phase == LocalGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        // phase work
        match self.major_phase {
            LocalGcPhase::Idle => Ok(GcProgress::Idle),
            LocalGcPhase::Mark => {
                // roots may have changed between incremental steps
                self.seed_major_roots(roots)?;
                let marked_bytes = self.mark_reachable_references_step(budget_bytes)?;

                // switch to sweep when mark work drains
                if self.major_trace_queue.is_empty() {
                    self.major_sweep_references = self.live_references()?;
                    self.major_sweep_cursor = 0;
                    self.major_phase = LocalGcPhase::Sweep;

                    // spend remaining budget in the sweep phase
                    if marked_bytes < budget_bytes {
                        let remaining_bytes = budget_bytes - marked_bytes;

                        return Ok(self.sweep_unreachable_references_step(remaining_bytes)?);
                    }
                }

                Ok(GcProgress::Active)
            }
            LocalGcPhase::Sweep => Ok(self.sweep_unreachable_references_step(budget_bytes)?),
        }
    }

    /// Seed the active major trace queue from explicit roots and pins.
    fn seed_major_roots<R>(&mut self, roots: &mut R) -> Result<(), R::Error>
    where
        R: RootSet,
    {
        // root slots
        roots.visit_root_slots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_major_reference(reference)
        })?;

        // pins
        let pins = self.pins.references().collect::<Vec<_>>();
        for reference in pins {
            self.enqueue_major_reference(reference)?;
        }

        Ok(())
    }

    /// Finish the active local major collection.
    fn finish_major_gc(&mut self) -> HeapResult<GcStats> {
        let stats =
            self.stats_after_collection(self.major_freed_allocations, self.major_freed_bytes);

        // publish cycle statistics and reset collector state
        self.gc.record_cycle(GcKind::Full, stats);
        self.major_phase = LocalGcPhase::Idle;
        self.major_trace_queue.clear();
        self.major_sweep_references.clear();
        self.major_sweep_cursor = 0;
        self.major_freed_allocations = 0;
        self.major_freed_bytes = 0;
        self.is_collecting = false;

        Ok(stats)
    }

    /// Sweep unreachable references within one byte budget.
    fn sweep_unreachable_references_step(&mut self, budget_bytes: usize) -> HeapResult<GcProgress> {
        let mut swept_bytes = 0usize;

        // sweep bounded live-reference candidates
        while swept_bytes < budget_bytes
            && self.major_sweep_cursor < self.major_sweep_references.len()
        {
            let reference = self.major_sweep_references[self.major_sweep_cursor];
            self.major_sweep_cursor += 1;

            let Some(location) = self.resolve_location(reference) else {
                continue;
            };
            swept_bytes += location.byte_len.max(1);

            // keep reachable references intact
            if self.is_marked_place(location.place)? {
                continue;
            }

            // free unreachable references and charge the reclaimed bytes
            self.free(reference)
                .map_err(|error| HeapError::HeapFreeFailed {
                    reference,
                    error: Box::new(error),
                })?;
            self.major_freed_allocations += 1;
            self.major_freed_bytes += location.byte_len as u64;
        }

        // finish once every candidate has been visited
        if self.major_sweep_cursor >= self.major_sweep_references.len() {
            return self.finish_major_gc().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Mark reachable heap references within one byte budget.
    fn mark_reachable_references_step(&mut self, budget_bytes: usize) -> HeapResult<usize> {
        let mut marked_bytes = 0usize;

        // trace bounded mark work
        while marked_bytes < budget_bytes {
            // claim the next bounded mark item
            let Some(work) = self.major_trace_queue.pop() else {
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
            self.major_trace_queue.push(LocalTraceWork::LargeRange {
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

            // large allocations must not monopolize one safepoint
            if matches!(location.place, HeapPlace::Large(_)) {
                self.major_trace_queue.push(LocalTraceWork::LargeRange {
                    reference,
                    start: 0,
                });

                return Ok(());
            }

            // smaller allocations are one mark item
            self.major_trace_queue
                .push(LocalTraceWork::Reference(reference));
        }

        Ok(())
    }

    /// Clear every collector mark bit in the live heap.
    pub(super) fn clear_mark_bits(&mut self) -> HeapResult<()> {
        // nursery marks
        self.young.marked.clear_all();
        for bits in &mut self.young.run_bits {
            bits.marked.clear_all();
        }

        // small-span marks
        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            span.clear_marks();
        }

        // large-allocation marks
        for allocation_index in 0..self.large.allocations.len() {
            let Some(allocation) = self.large.allocations.get_mut(allocation_index) else {
                let allocation_id = LargeAllocationId::new(allocation_index as u64 + 1);

                return Err(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                });
            };

            allocation.is_marked = false;
        }

        Ok(())
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

                Ok(span.marked.contains(slot.slot_index()))
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                Ok(allocation.is_marked)
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
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                span.marked.set(slot.slot_index());
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                allocation.is_marked = true;
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
        // current live heap usage
        let (live_allocations, allocated_bytes) = self.live_allocated_usage();

        GcStats {
            freed_allocations,
            live_allocations,
            freed_bytes,
            allocated_bytes,
            retained_bytes: self.retained_bytes(),
        }
    }
}
