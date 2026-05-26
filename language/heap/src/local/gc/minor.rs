use super::Promotion;
use crate::local::space::{
    GcKind, GcStats, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocationId, YoungPlace,
    YoungRunCursor,
};
use crate::{
    HeapError, HeapReference, HeapResult, RootSet, RootSlot, ScanSource, TraceQueue,
    scan_heap_references, scan_heap_references_in_range, slot_trace_map,
};

/// Collector queue for heap references.
type HeapTraceQueue = TraceQueue<HeapReference>;

impl HeapSpace {
    /// Perform one young-generation collection over mutable heap roots.
    pub fn collect_minor<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSet,
    {
        // reject overlapping collection work
        if self.is_collecting {
            return Err(HeapError::HeapCollectionActive.into());
        }

        // prepare reusable collection state
        let queue = std::mem::take(&mut self.trace_queue);
        let mut pending = queue;
        let mut promotions = Vec::new();
        let pinned = self.pins.references().collect::<Vec<_>>();

        // begin the new cycle
        self.clear_mark_bits()?;
        pending.clear();
        self.is_collecting = true;

        // trace every reachable young allocation from roots and remembered mature writes
        let result =
            self.collect_minor_cycle(roots, pinned.iter().copied(), &mut pending, &mut promotions);

        self.trace_queue = pending;
        self.is_collecting = false;

        result
    }

    /// Perform one prepared minor collection cycle.
    fn collect_minor_cycle<R>(
        &mut self,
        roots: &mut R,
        pinned: impl Iterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
        promotions: &mut Vec<Promotion>,
    ) -> Result<GcStats, R::Error>
    where
        R: RootSet,
    {
        // root slots
        roots.visit_root_slots(&mut |slot: RootSlot<'_>| {
            let Some(reference) = slot.load_heap_reference()? else {
                return Ok(());
            };

            self.enqueue_young_reference(reference, pending)
        })?;

        // trace young graph
        self.mark_reachable_young_references(pinned, pending)?;
        let (freed_allocations, freed_bytes) = self.promote_or_free_young_references(promotions)?;

        // rewrite roots and traced mature payloads before resetting young space
        self.rewrite_promoted_references(roots, promotions)?;

        // reset the young space after promotion and sweeping
        self.reset_young_space()?;

        // finalize the completed minor-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc.record_cycle(GcKind::Minor, stats);

        Ok(stats)
    }

    /// Promote or free every young reference seen during one minor collection.
    fn promote_or_free_young_references(
        &mut self,
        promotions: &mut Vec<Promotion>,
    ) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every live young allocation directly
        for reference in self.live_references()? {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            // stage reachable young allocations for promotion
            if self.is_marked_place(location.place)? {
                let result = match location.place {
                    HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                        self.stage_young_promotion(reference, first_offset, promotions)
                    }
                    HeapPlace::Young(YoungPlace::Slot(slot)) => {
                        self.stage_young_run_promotion(reference, slot, promotions)
                    }
                    _ => continue,
                };
                if let Err(error) = result {
                    self.discard_young_promotions(promotions)?;

                    return Err(error);
                }

                continue;
            }

            // ignore mature allocations from the live reference walk
            if !matches!(
                location.place,
                HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_))
            ) {
                continue;
            }

            // otherwise free unreachable young allocations
            match self.free(reference) {
                Ok(()) => {}
                Err(error) => {
                    self.discard_young_promotions(promotions)?;

                    return Err(HeapError::HeapFreeFailed {
                        reference,
                        error: Box::new(error),
                    });
                }
            };

            freed_allocations += 1;
            freed_bytes += location.byte_len as u64;
        }

        // rewrite references only after every target has been staged
        if let Err(error) = self.commit_young_promotions(promotions) {
            self.discard_young_promotions(promotions)?;

            return Err(error);
        }

        Ok((freed_allocations, freed_bytes))
    }

    /// Reset the young space after one collection cycle.
    fn reset_young_space(&mut self) -> HeapResult<()> {
        let allocator = self.allocator().clone();
        let next_pages = self
            .page_run_cache
            .allocate_pages(&allocator, self.young.capacity_bytes)?;
        let previous_pages = self.young.pages;

        // clear the old nursery page map before its pages reenter the cache
        self.unmap_page_run(0, &previous_pages);

        // release the old nursery pages once the page-map table is clean
        if let Err(error) = self
            .page_run_cache
            .release_page_run(&allocator, previous_pages)
        {
            self.map_page_run(0, &previous_pages, |logical_page_index| {
                HeapPageMapEntry::Young { logical_page_index }
            });
            self.page_run_cache
                .release_page_run(&allocator, next_pages)?;

            return Err(error);
        }

        // publish the fresh nursery state
        self.young.generation += 1;
        self.young.next_offset = self.young.allocation_alignment_bytes;
        self.young.mapped_until = 0;
        self.young.pages = next_pages;
        self.young.starts.clear_all();
        self.young.live.clear_all();
        self.young.marked.clear_all();
        self.young.local_reference_bits.clear_all();
        self.young.shared_reference_bits.clear_all();
        self.young.runs.clear();
        self.young.run_bits.clear();
        self.young.run_forwarded.clear();
        self.young.run_buckets.fill(None);
        self.young.run_cursor = YoungRunCursor::inactive();
        self.young.page_runs.fill(None);

        let next_pages = self.young.pages;

        self.map_page_run(0, &next_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        Ok(())
    }

    /// Mark every reachable young reference.
    fn mark_reachable_young_references(
        &mut self,
        roots: impl IntoIterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        // seed the work queue from explicit young roots
        for reference in roots {
            self.enqueue_young_reference(reference, pending)?;
        }

        // seed the work queue from remembered mature writes
        self.enqueue_dirty_young_references(pending)?;

        // drain the young reference queue
        while let Some(reference) = pending.pop() {
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
                self.enqueue_young_reference(reference, pending)?;
            }
        }

        Ok(())
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

    /// Queue every young reference discovered from remembered mature writes.
    fn enqueue_dirty_young_references(&mut self, pending: &mut HeapTraceQueue) -> HeapResult<()> {
        // snapshot dirty sets before scanning through self
        let dirty_spans = self.dirty_spans.clone();
        let dirty_large_allocations = self.dirty_large_allocations.clone();

        // scan each queued dirty span
        for span_index in dirty_spans {
            self.enqueue_dirty_span_references(span_index, pending)?;
        }

        // scan each queued dirty large allocation
        for allocation_id in dirty_large_allocations {
            self.enqueue_dirty_large_allocation_references(allocation_id, pending)?;
        }

        self.dirty_spans.clear();
        self.dirty_large_allocations.clear();

        Ok(())
    }

    /// Queue every young reference discovered from one dirty span.
    fn enqueue_dirty_span_references(
        &mut self,
        span_index: usize,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Span(span_index),
                error: Box::new(HeapError::MissingSpan { span_index }),
            });
        };
        let occupied = span.occupied.clone();
        let slot_count = span.slot_count;
        let size_class = span.class.size_class;
        let local_reference_bits = span.local_reference_bits.clone();
        let shared_reference_bits = span.shared_reference_bits.clone();
        let span_offset = span.first_offset;
        let dirty_cards = span.dirty_cards.clone();

        // scan each dirty card window against each occupied slot
        for (card_start, card_len) in dirty_cards.dirty_ranges() {
            // derive the slot range overlapped by this dirty card
            let card_end = card_start + card_len;
            let first_slot = card_start / size_class;
            let last_slot = (card_end - 1) / size_class;
            let end_slot = (last_slot + 1).min(slot_count);

            // scan each occupied slot in the dirty window
            for slot_index in first_slot..end_slot {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let trace_map = slot_trace_map(
                    &local_reference_bits,
                    &shared_reference_bits,
                    slot_index,
                    size_class,
                    size_class,
                );
                if !trace_map.has_local_reference() {
                    continue;
                }

                // intersect dirty-card bytes with this slot payload
                let slot_start = size_class * slot_index;
                let slot_end = slot_start + size_class;
                let overlap_start = card_start.max(slot_start);
                let overlap_end = card_end.min(slot_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                let local_start = overlap_start - slot_start;
                let local_len = overlap_end - overlap_start;
                let mut references = Vec::new();

                // scan only the dirty byte intersection
                let base_address = self.mapping.base_address() + span_offset + slot_start;
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

                // enqueue discovered young references
                for reference in references {
                    self.enqueue_young_reference(reference, pending)?;
                }
            }
        }

        // clear only after the scan succeeds
        if let Some(span) = self.span_mut(span_index) {
            span.dirty_cards.clear();
            span.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Queue every young reference discovered from one dirty large allocation.
    fn enqueue_dirty_large_allocation_references(
        &mut self,
        allocation_id: LargeAllocationId,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        let Some(allocation) = self.large_allocation(allocation_id) else {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::LargeAllocation(allocation_id.id()),
                error: Box::new(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                }),
            });
        };
        let allocation_offset = allocation.first_offset;
        let dirty_cards = allocation.dirty_cards.clone();
        let trace_map = allocation.trace_map.clone();

        // scan each dirty card window directly against the large allocation
        for (card_start, card_len) in dirty_cards.dirty_ranges() {
            let mut references = Vec::new();

            // scan only the dirty byte range
            let base_address = self.mapping.base_address() + allocation_offset;
            scan_heap_references_in_range(
                &trace_map,
                card_start,
                card_len,
                base_address,
                &mut references,
            )
            .map_err(|error| HeapError::HeapScanFailed {
                source: ScanSource::LargeAllocation(allocation_id.id()),
                error: Box::new(error),
            })?;

            // enqueue discovered young references
            for reference in references {
                self.enqueue_young_reference(reference, pending)?;
            }
        }

        // clear only after the scan succeeds
        if let Some(allocation) = self.large_allocation_mut(allocation_id) {
            allocation.dirty_cards.clear();
            allocation.is_dirty_queued = false;
        }

        Ok(())
    }
}
