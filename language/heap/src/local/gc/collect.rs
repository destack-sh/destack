use std::collections::BTreeMap;

use destack_mir::ReferenceMap;

use super::Promotion;
use crate::local::space::{
    GcKind, GcStats, HeapLocation, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocationId,
    LocalGcPhase, LocalTraceWork,
};
use crate::{
    AccountingRegion, GcProgress, HeapError, HeapReference, HeapResult, RootSlot, RootSlots,
    ScanSource, SharedHeapReference, TraceQueue, slot_reference_map,
    visit_heap_references_in_reader, visit_heap_references_in_reader_range,
    visit_shared_references_in_reader,
};

/// Collector queue for heap references.
type HeapTraceQueue = TraceQueue<HeapReference>;

impl HeapSpace {
    /// Return whether one local major collection is active.
    pub(crate) fn major_gc_active(&self) -> bool {
        self.major_phase != LocalGcPhase::Idle
    }

    /// Return the currently live heap references.
    pub fn live_references(&self) -> HeapResult<Vec<HeapReference>> {
        let mut references = Vec::new();

        for allocation_index in 0..self.young.ranges.len() {
            if !self.young.live.contains(allocation_index) {
                continue;
            }
            let allocation = &self.young.ranges[allocation_index];

            let allocation_offset = self.young_range_offset(allocation);
            let base_address =
                self.allocator()
                    .page_view_ptr(&self.young.pages, allocation_offset)? as usize;

            references.push(HeapReference::new(base_address));
        }

        for span in self.small.spans.iter() {
            for slot_index in 0..span.slot_count {
                if !span.occupied.contains(slot_index) {
                    continue;
                }

                let slot_offset = span.class.size_class.checked_mul(slot_index).ok_or(
                    HeapError::InvariantOverflow {
                        context: "heap slot base offset",
                    },
                )?;
                let base_address =
                    self.allocator().page_view_ptr(&span.pages, slot_offset)? as usize;

                references.push(HeapReference::new(base_address));
            }
        }

        for allocation in self.large.allocations.iter() {
            if !allocation.is_live {
                continue;
            }

            let base_address = self.allocator().page_view_ptr(&allocation.pages, 0)? as usize;
            references.push(HeapReference::new(base_address));
        }

        Ok(references)
    }

    /// Start one incremental local-to-shared edge scan.
    pub(crate) fn start_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = true;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
    }

    /// Return whether the current local-to-shared edge scan is fully drained.
    pub(crate) fn shared_edge_scan_idle(&self) -> bool {
        !self.is_scanning_shared_edges
            || (self.shared_edge_cursor >= self.shared_edge_roots.len()
                && self.shared_edge_queue.is_empty())
    }

    /// Finish the current local-to-shared edge scan.
    pub(crate) fn finish_shared_edge_scan(&mut self) {
        self.is_scanning_shared_edges = false;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();
        self.compact_shared_edge_roots();
    }

    /// Scan bounded local-to-shared edge work into the provided root buffer.
    pub(crate) fn scan_shared_edge_step(
        &mut self,
        roots: &mut Vec<SharedHeapReference>,
        step_budget: usize,
    ) -> HeapResult<usize> {
        if !self.is_scanning_shared_edges || step_budget == 0 {
            return Ok(0);
        }

        let mut work_done = 0usize;

        // drain queued rescans first
        while work_done < step_budget {
            let Some(reference) = self.shared_edge_queue.pop() else {
                break;
            };

            self.shared_edge_pending.remove(&reference);
            self.trace_shared_edges(reference, roots)?;
            work_done += 1;
        }

        // then continue the tracked shared-edge walk
        while work_done < step_budget {
            let Some(reference) = self.next_shared_edge_root()? else {
                break;
            };

            self.trace_shared_edges(reference, roots)?;
            work_done += 1;
        }

        Ok(work_done)
    }

    /// Queue one local reference for one later shared-edge rescan.
    pub(crate) fn queue_shared_reference(&mut self, reference: HeapReference) -> HeapResult<()> {
        if !self.is_scanning_shared_edges || !self.reference_has_shared_roots(reference)? {
            return Ok(());
        }

        if !self.shared_edge_pending.insert(reference) {
            return Ok(());
        }

        self.shared_edge_queue.push(reference);

        Ok(())
    }

    /// Publish one allocation to an active local major cycle.
    pub(crate) fn publish_major_allocation(
        &mut self,
        reference: HeapReference,
        place: HeapPlace,
    ) -> HeapResult<()> {
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
        let reference_map = self.place_reference_map(place)?;
        if !reference_map.has_local_reference() {
            return Ok(());
        }

        let location = HeapLocation {
            place,
            base: reference,
            byte_offset: 0,
            byte_len: self.place_byte_len(place)?,
        };
        let scan_len = byte_len.min(location.byte_len.saturating_sub(byte_offset));
        let mut references = Vec::new();
        let result = visit_heap_references_in_reader_range(
            &reference_map,
            byte_offset,
            scan_len,
            |start, buffer| self.fill_location_bytes(location, start, buffer),
            |reference| {
                references.push(reference);
            },
        );

        if let Err(error) = result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        for reference in references {
            self.enqueue_major_reference(reference)?;
        }

        Ok(())
    }

    /// Return the next tracked local reference that may contain shared edges.
    fn next_shared_edge_root(&mut self) -> HeapResult<Option<HeapReference>> {
        while self.shared_edge_cursor < self.shared_edge_roots.len() {
            let index = self.shared_edge_cursor;
            self.shared_edge_cursor += 1;

            let Some(reference) = self.shared_edge_roots.get(index).copied() else {
                return Ok(None);
            };

            if !reference.is_null() {
                return Ok(Some(reference));
            }
        }

        Ok(None)
    }

    /// Return whether one live local reference may contain shared heap roots.
    pub(crate) fn reference_has_shared_roots(&self, reference: HeapReference) -> HeapResult<bool> {
        let Some(location) = self.resolve_location(reference) else {
            return Ok(false);
        };
        let reference_map = self.place_reference_map(location.place)?;

        Ok(reference_map.has_shared_reference())
    }

    /// Trace shared heap roots from one heap reference.
    fn trace_shared_edges(
        &mut self,
        reference: HeapReference,
        roots: &mut Vec<SharedHeapReference>,
    ) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Ok(());
        };
        let reference_map = self.place_reference_map(location.place).map_err(|error| {
            HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            }
        })?;

        if !reference_map.has_shared_reference() {
            return Ok(());
        }

        let result = visit_shared_references_in_reader(
            &reference_map,
            |start, buffer| self.fill_location_bytes(location, start, buffer),
            |reference: SharedHeapReference| {
                if !reference.is_null() {
                    roots.push(reference);
                }
            },
        );

        if let Err(error) = result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        Ok(())
    }

    /// Perform one young-generation collection over mutable heap roots.
    pub fn collect_minor<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSlots,
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

    /// Perform one full heap collection over mutable heap roots.
    pub fn collect_full<R>(&mut self, roots: &mut R) -> Result<GcStats, R::Error>
    where
        R: RootSlots,
    {
        let _minor = self.collect_minor(roots)?;

        self.start_major_gc(roots)?;

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

    /// Perform one prepared minor collection cycle.
    fn collect_minor_cycle<R>(
        &mut self,
        roots: &mut R,
        pinned: impl Iterator<Item = HeapReference>,
        pending: &mut HeapTraceQueue,
        promotions: &mut Vec<Promotion>,
    ) -> Result<GcStats, R::Error>
    where
        R: RootSlots,
    {
        // root slots
        roots.visit_root_slots(&mut |slot: RootSlot<'_>| {
            let reference = slot.load()?;

            self.enqueue_young_reference(reference, pending)
        })?;

        // pins
        self.mark_reachable_young_references(pinned, pending)?;
        let (freed_allocations, freed_bytes) = self.promote_or_free_young_references(promotions)?;

        // rewrite roots and traced mature payloads before resetting young space
        self.rewrite_promoted_references(roots, promotions)?;

        // reset the young space after promotion and sweeping
        self.reset_young_space()?;

        // finalize the completed minor-cycle statistics
        let stats = self.stats_after_collection(freed_allocations, freed_bytes);

        self.gc.record_cycle(GcKind::Minor, stats)?;

        Ok(stats)
    }

    /// Start one local major collection.
    pub(crate) fn start_major_gc<R>(&mut self, roots: &mut R) -> Result<(), R::Error>
    where
        R: RootSlots,
    {
        if self.is_collecting {
            return Err(HeapError::HeapCollectionActive.into());
        }

        self.clear_mark_bits()?;
        self.major_trace_queue.clear();
        self.major_sweep_references.clear();
        self.major_sweep_cursor = 0;
        self.major_freed_allocations = 0;
        self.major_freed_bytes = 0;
        self.major_phase = LocalGcPhase::Mark;
        self.is_collecting = true;

        self.seed_major_roots(roots)?;

        Ok(())
    }

    /// Perform bounded work for one active local major collection.
    pub(crate) fn step_major_gc<R>(
        &mut self,
        roots: &mut R,
        step_budget: usize,
    ) -> Result<GcProgress, R::Error>
    where
        R: RootSlots,
    {
        if step_budget == 0 || self.major_phase == LocalGcPhase::Idle {
            return Ok(GcProgress::Idle);
        }

        match self.major_phase {
            LocalGcPhase::Idle => Ok(GcProgress::Idle),
            LocalGcPhase::Mark => {
                self.seed_major_roots(roots)?;
                let mark_work = self.mark_reachable_references_step(step_budget)?;

                if self.major_trace_queue.is_empty() {
                    self.major_sweep_references = self.live_references()?;
                    self.major_sweep_cursor = 0;
                    self.major_phase = LocalGcPhase::Sweep;

                    let sweep_work = step_budget.saturating_sub(mark_work);
                    if sweep_work > 0 {
                        return Ok(self.sweep_unreachable_references_step(sweep_work)?);
                    }
                }

                Ok(GcProgress::Active)
            }
            LocalGcPhase::Sweep => Ok(self.sweep_unreachable_references_step(step_budget)?),
        }
    }

    /// Seed the active major trace queue from explicit roots and pins.
    fn seed_major_roots<R>(&mut self, roots: &mut R) -> Result<(), R::Error>
    where
        R: RootSlots,
    {
        // root slots
        roots.visit_root_slots(&mut |slot: RootSlot<'_>| {
            let reference = slot.load()?;

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

        self.gc.record_cycle(GcKind::Full, stats)?;
        self.major_phase = LocalGcPhase::Idle;
        self.major_trace_queue.clear();
        self.major_sweep_references.clear();
        self.major_sweep_cursor = 0;
        self.major_freed_allocations = 0;
        self.major_freed_bytes = 0;
        self.is_collecting = false;

        Ok(stats)
    }

    /// Rewrite roots and traced mature payloads through one completed promotion set.
    fn rewrite_promoted_references<R>(
        &mut self,
        roots: &mut R,
        promotions: &[Promotion],
    ) -> Result<(), R::Error>
    where
        R: RootSlots,
    {
        if promotions.is_empty() {
            return Ok(());
        }

        let references = self.promotion_references(promotions)?;

        // rewrite root slots first
        roots.visit_root_slots(&mut |mut slot: RootSlot<'_>| {
            let reference = slot.load()?;

            if let Some(next_reference) = references.get(&reference).copied() {
                slot.store(next_reference)?;
            }

            Ok(())
        })?;

        // then rewrite every pinned reference
        self.pins.rewrite(&references)?;

        // then rewrite every mature payload that may still contain young references
        self.rewrite_live_heap_references(&references)?;

        // finally rebuild the auxiliary shared-edge tracking over the new stable refs
        self.rebuild_shared_edge_roots()?;
        self.shared_edge_cursor = 0;
        self.shared_edge_queue.clear();
        self.shared_edge_pending.clear();

        Ok(())
    }

    /// Return the completed old to new reference map for one promotion set.
    fn promotion_references(
        &self,
        promotions: &[Promotion],
    ) -> HeapResult<BTreeMap<HeapReference, HeapReference>> {
        let mut references = BTreeMap::new();

        for promotion in promotions {
            let next_reference = self.base_reference(promotion.target)?;
            references.insert(promotion.reference, next_reference);
        }

        Ok(references)
    }

    /// Rewrite every live mature payload through one completed promotion map.
    fn rewrite_live_heap_references(
        &mut self,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        let live_references = self.live_references()?;

        for reference in live_references {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            if matches!(location.place, HeapPlace::Young { .. }) {
                continue;
            }

            let reference_map = self.place_reference_map(location.place)?;

            if !reference_map.has_local_reference() {
                continue;
            }

            self.rewrite_location_heap_references(location, &reference_map, references)?;
        }

        Ok(())
    }

    /// Rewrite one traced heap payload through one completed promotion map.
    fn rewrite_location_heap_references(
        &mut self,
        location: HeapLocation,
        reference_map: &ReferenceMap,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        match reference_map {
            ReferenceMap::None => {}
            ReferenceMap::Reference { local_offsets, .. } => {
                for offset in local_offsets.iter().copied() {
                    self.rewrite_location_heap_reference_word(
                        location,
                        offset as usize,
                        references,
                    )?;
                }
            }
            ReferenceMap::RepeatedReference {
                count,
                stride,
                local_offsets,
                ..
            } => {
                for index in 0..*count as usize {
                    let element_start = index.checked_mul(*stride as usize).ok_or(
                        HeapError::InvariantOverflow {
                            context: "repeated local reference rewrite",
                        },
                    )?;

                    for offset in local_offsets.iter().copied() {
                        let start = element_start.checked_add(offset as usize).ok_or(
                            HeapError::InvariantOverflow {
                                context: "repeated local reference rewrite",
                            },
                        )?;

                        self.rewrite_location_heap_reference_word(location, start, references)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Rewrite one direct heap-reference word inside one traced payload.
    fn rewrite_location_heap_reference_word(
        &mut self,
        location: HeapLocation,
        start: usize,
        references: &BTreeMap<HeapReference, HeapReference>,
    ) -> HeapResult<()> {
        let mut window = [0u8; HeapReference::BYTE_LEN];

        self.fill_location_bytes(location, start, &mut window)?;

        let reference = HeapReference::from_bits(usize::from_le_bytes(window));
        let Some(next_reference) = references.get(&reference).copied() else {
            return Ok(());
        };
        let next_bytes = next_reference.bits().to_le_bytes();

        self.write_location_bytes(location, start, &next_bytes)
    }

    /// Promote one young reference into mature space.
    pub(crate) fn promote_reference(
        &mut self,
        reference: HeapReference,
    ) -> HeapResult<HeapReference> {
        // resolve the current live location
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // mature references already have stable addresses
        let HeapPlace::Young { first_offset } = location.place else {
            return Ok(reference);
        };

        // stage the young to mature relocation
        let mut promotions = Vec::with_capacity(1);

        self.stage_young_promotion(reference, first_offset, &mut promotions)?;

        // publish the mature location only after staging succeeds
        if let Err(error) = self.commit_young_promotions(&promotions) {
            self.discard_young_promotions(&promotions)?;

            return Err(error);
        }
        self.charge_young_promotions(&promotions)?;

        // retire the old nursery source after the promoted reference points at mature place
        let Some((allocation_index, _allocation)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::MissingYoungRange { first_offset });
        };
        self.young.live.clear(allocation_index);

        // remember the new mature location conservatively
        let Some(promotion) = promotions.first().copied() else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        let promoted_reference = self.base_reference(promotion.target)?;

        self.write_barrier_location(
            HeapLocation {
                place: promotion.target,
                base: promoted_reference,
                byte_offset: 0,
                byte_len: location.byte_len,
            },
            0,
            location.byte_len,
        )?;

        // keep active shared-edge scans aware of the now-mature allocation
        if self.is_scanning_shared_edges
            && self
                .place_reference_map(promotion.target)?
                .has_shared_reference()
        {
            self.queue_shared_reference(promoted_reference)?;
        }

        Ok(promoted_reference)
    }

    /// Stage one live young allocation relocation into mature space.
    fn stage_young_promotion(
        &mut self,
        reference: HeapReference,
        first_offset: usize,
        promotions: &mut Vec<Promotion>,
    ) -> HeapResult<()> {
        let Some((_allocation_index, allocation)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(HeapError::MissingYoungRange { first_offset }),
            });
        };
        let allocation = allocation.clone();

        // resolve the shared source range once before relocating the allocation
        let young_offset = self.young_range_offset(&allocation);
        let young_pages = self.young.pages.clone();
        let reference_map = self
            .young_range_reference_map(first_offset)
            .map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error),
            })?;

        // prefer one mature small slot when the payload fits one size class
        let location = if self
            .small
            .size_classes
            .class_index_for(allocation.byte_len)
            .is_some()
        {
            let slot = self
                .allocate_small_payload_from_page_view(
                    &young_pages,
                    young_offset,
                    allocation.byte_len,
                    &reference_map,
                    false,
                )
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;
            let Some(slot) = slot else {
                return Err(HeapError::HeapPromotionUnavailableSmallSlot {
                    reference,
                    byte_len: allocation.byte_len,
                });
            };

            HeapPlace::Small(slot)
        }
        // otherwise copy the payload directly into one mature large allocation
        else {
            let mut pages = self
                .allocate_page_view_zeroed(allocation.byte_len)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            let allocator = self.allocator().clone();

            // copy the young payload into the new large allocation
            if let Err(error) = allocator.copy_bytes_between_page_views(
                &young_pages,
                young_offset,
                &mut pages,
                0,
                allocation.byte_len,
            ) {
                self.release_page_view(pages)
                    .map_err(|error| HeapError::HeapPromotionFailed {
                        reference,
                        error: Box::new(error),
                    })?;

                return Err(HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                });
            }

            let allocation_id = self
                .insert_large_allocation(allocation.byte_len, pages, reference_map, false)
                .map_err(|error| HeapError::HeapPromotionFailed {
                    reference,
                    error: Box::new(error),
                })?;

            HeapPlace::Large(allocation_id)
        };

        promotions.push(Promotion {
            reference,
            source: HeapPlace::Young { first_offset },
            target: location,
        });

        Ok(())
    }

    /// Verify every staged young relocation still refers to the staged source.
    fn commit_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions {
            let reference = promotion.reference;
            let Some(location) = self.resolve_location(reference) else {
                return Err(HeapError::InvalidHeapReference { reference });
            };
            if location.place != promotion.source {
                return Err(HeapError::InvalidHeapReference { reference });
            }
        }

        Ok(())
    }

    /// Discard every staged mature relocation target.
    fn discard_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions.iter().rev() {
            let reference = promotion.reference;
            let result = match promotion.target {
                HeapPlace::Young { first_offset } => {
                    Err(HeapError::MissingYoungRange { first_offset })
                }
                HeapPlace::Small(slot) => self.release_small_slot(slot),
                HeapPlace::Large(allocation_id) => {
                    let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    };

                    if !allocation.is_live {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    }

                    let pages = allocation.pages.clone();
                    allocation.retire();
                    self.large
                        .free_large_allocation_ids
                        .push(allocation_id.id());

                    // release the unpublished target pages after discarding the slot
                    self.release_page_view(pages)?;

                    Ok(())
                }
            };

            result.map_err(|error| HeapError::HeapPromotionFailed {
                reference,
                error: Box::new(error),
            })?;
        }

        Ok(())
    }

    /// Promote or free every young reference seen during one minor collection.
    fn promote_or_free_young_references(
        &mut self,
        promotions: &mut Vec<Promotion>,
    ) -> Result<(usize, u64), HeapError> {
        let mut freed_allocations = 0usize;
        let mut freed_bytes = 0u64;

        // walk every live local allocation directly
        for reference in self.live_references()? {
            let Some(location) = self.resolve_location(reference) else {
                continue;
            };
            let HeapPlace::Young { first_offset } = location.place else {
                continue;
            };

            if self.is_marked_place(location.place)? {
                if let Err(error) = self.stage_young_promotion(reference, first_offset, promotions)
                {
                    self.discard_young_promotions(promotions)?;

                    return Err(error);
                }

                continue;
            }

            // otherwise free unreachable young allocations
            let did_free = match self.free(reference) {
                Ok(did_free) => did_free,
                Err(error) => {
                    self.discard_young_promotions(promotions)?;

                    return Err(HeapError::HeapFreeFailed {
                        reference,
                        error: Box::new(error),
                    });
                }
            };

            if did_free {
                freed_allocations =
                    freed_allocations
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "young gc freed allocation count",
                        })?;
                freed_bytes = freed_bytes.checked_add(location.byte_len as u64).ok_or(
                    HeapError::InvariantOverflow {
                        context: "young gc freed bytes",
                    },
                )?;
            }
        }

        // rewrite references only after every target has been staged
        if let Err(error) = self.commit_young_promotions(promotions) {
            self.discard_young_promotions(promotions)?;

            return Err(error);
        }
        self.charge_young_promotions(promotions)?;

        Ok((freed_allocations, freed_bytes))
    }

    /// Reconcile allocation accounting after young allocations move to mature space.
    fn charge_young_promotions(&mut self, promotions: &[Promotion]) -> HeapResult<()> {
        for promotion in promotions {
            let HeapPlace::Young { first_offset } = promotion.source else {
                return Err(HeapError::InvalidHeapReference {
                    reference: promotion.reference,
                });
            };
            let Some((_allocation_index, allocation)) = self.young_range_by_offset(first_offset)
            else {
                return Err(HeapError::MissingYoungRange { first_offset });
            };
            let source_len = allocation.byte_len;
            let target_len = self.place_byte_len(promotion.target)?;

            self.usage
                .resize(source_len, target_len, AccountingRegion::Heap);
        }

        Ok(())
    }

    /// Sweep bounded unreachable references during one local major collection.
    fn sweep_unreachable_references_step(&mut self, step_budget: usize) -> HeapResult<GcProgress> {
        let mut work_done = 0usize;

        while work_done < step_budget && self.major_sweep_cursor < self.major_sweep_references.len()
        {
            let reference = self.major_sweep_references[self.major_sweep_cursor];
            self.major_sweep_cursor += 1;
            work_done += 1;

            let Some(location) = self.resolve_location(reference) else {
                continue;
            };

            // keep reachable references intact
            if self.is_marked_place(location.place)? {
                continue;
            }

            // free unreachable references and charge the reclaimed bytes
            if self
                .free(reference)
                .map_err(|error| HeapError::HeapFreeFailed {
                    reference,
                    error: Box::new(error),
                })?
            {
                self.major_freed_allocations = self.major_freed_allocations.checked_add(1).ok_or(
                    HeapError::InvariantOverflow {
                        context: "full gc freed allocation count",
                    },
                )?;
                self.major_freed_bytes = self
                    .major_freed_bytes
                    .checked_add(location.byte_len as u64)
                    .ok_or(HeapError::InvariantOverflow {
                        context: "full gc freed bytes",
                    })?;
            }
        }

        if self.major_sweep_cursor >= self.major_sweep_references.len() {
            return self.finish_major_gc().map(GcProgress::Complete);
        }

        Ok(GcProgress::Active)
    }

    /// Reset the young space after one collection cycle.
    fn reset_young_space(&mut self) -> HeapResult<()> {
        let allocator = self.allocator().clone();
        let next_pages = self
            .page_run_cache
            .allocate_zeroed(&allocator, self.young.capacity_bytes)
            .map_err(|error| HeapError::HeapYoungResetFailed {
                error: Box::new(error),
            })?;
        let previous_pages = self.young.pages.clone();

        // remove the old nursery ownership before its pages reenter the allocator cache
        self.unmap_page_view(&previous_pages)?;

        // release the old nursery pages once the page-map table is clean
        if let Err(error) = self
            .page_run_cache
            .release_page_view(&allocator, previous_pages.clone())
        {
            self.map_page_view(&previous_pages, |logical_page_index| {
                HeapPageMapEntry::Young { logical_page_index }
            })?;
            self.page_run_cache
                .release_page_view(&allocator, next_pages)?;

            return Err(HeapError::HeapYoungResetFailed {
                error: Box::new(error),
            });
        }

        // publish the fresh nursery state
        self.young.generation =
            self.young
                .generation
                .checked_add(1)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap young generation",
                })?;
        self.young.next_offset = 0;
        self.young.pages = next_pages;
        self.young.ranges.clear();
        self.young.live.clear_all();
        self.young.marked.clear_all();
        self.young.local_reference_bits.clear_all();
        self.young.shared_reference_bits.clear_all();

        let next_pages = self.young.pages.clone();

        self.map_page_view(&next_pages, |logical_page_index| HeapPageMapEntry::Young {
            logical_page_index,
        })
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

            if !matches!(location.place, HeapPlace::Young { .. }) {
                continue;
            }

            let reference_map = self.place_reference_map(location.place).map_err(|error| {
                HeapError::HeapScanFailed {
                    source: ScanSource::Reference(reference),
                    error: Box::new(error),
                }
            })?;

            let mut references = Vec::new();

            // enqueue every non-null young edge discovered in this payload
            let trace_result = visit_heap_references_in_reader(
                &reference_map,
                |start, buffer| self.read_bytes_into(reference, start, buffer),
                |reference: HeapReference| {
                    references.push(reference);
                },
            );

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

    /// Mark bounded reachable heap references from the active major queue.
    fn mark_reachable_references_step(&mut self, step_budget: usize) -> HeapResult<usize> {
        let mut work_done = 0usize;

        while work_done < step_budget {
            // claim the next bounded mark item
            let Some(work) = self.major_trace_queue.pop() else {
                break;
            };

            match work {
                // large allocations are scanned page by page
                LocalTraceWork::LargeRange { reference, start } => {
                    self.trace_large_range(reference, start)?;
                    work_done += 1;

                    continue;
                }

                // small and young allocations are scanned as one mark item
                LocalTraceWork::Reference(reference) => {
                    let Some(location) = self.resolve_location(reference) else {
                        return Err(HeapError::InvalidHeapReference { reference });
                    };

                    work_done += 1;

                    let reference_map =
                        self.place_reference_map(location.place).map_err(|error| {
                            HeapError::HeapScanFailed {
                                source: ScanSource::Reference(reference),
                                error: Box::new(error),
                            }
                        })?;

                    let mut references = Vec::new();

                    // collect every local edge discovered in this payload
                    let trace_result = visit_heap_references_in_reader(
                        &reference_map,
                        |start, buffer| self.read_bytes_into(reference, start, buffer),
                        |reference: HeapReference| {
                            references.push(reference);
                        },
                    );

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

        Ok(work_done)
    }

    /// Trace one page-sized range from one local large allocation.
    fn trace_large_range(&mut self, reference: HeapReference, start: usize) -> HeapResult<()> {
        // resolve and verify the large allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let HeapPlace::Large(_) = location.place else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // skip empty ranges and noscan payloads
        let reference_map = self.place_reference_map(location.place).map_err(|error| {
            HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            }
        })?;
        if !reference_map.has_local_reference() || start >= location.byte_len {
            return Ok(());
        }

        // scan at most one allocator page
        let range_len = self.allocator().page_bytes().min(location.byte_len - start);
        let mut references = Vec::new();
        let trace_result = visit_heap_references_in_reader_range(
            &reference_map,
            start,
            range_len,
            |start, buffer| self.read_bytes_into(reference, start, buffer),
            |reference: HeapReference| {
                references.push(reference);
            },
        );

        if let Err(error) = trace_result {
            return Err(HeapError::HeapScanFailed {
                source: ScanSource::Reference(reference),
                error: Box::new(error),
            });
        }

        // enqueue discovered local edges after the read borrow ends
        for reference in references {
            if !reference.is_null() && self.resolve_location(reference).is_none() {
                return Err(HeapError::InvalidHeapReference { reference });
            }

            self.enqueue_major_reference(reference)?;
        }

        // continue this large allocation on a later step
        let next_start = start
            .checked_add(range_len)
            .ok_or(HeapError::TraceOffsetOverflow {
                start,
                width: range_len,
            })?;
        if next_start < location.byte_len {
            self.major_trace_queue.push(LocalTraceWork::LargeRange {
                reference,
                start: next_start,
            });
        }

        Ok(())
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
            let reference_map = self.place_reference_map(location.place)?;
            if !reference_map.has_local_reference() {
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

    /// Queue one young reference when it currently points into the young space.
    fn enqueue_young_reference(
        &mut self,
        reference: HeapReference,
        pending: &mut HeapTraceQueue,
    ) -> HeapResult<()> {
        if reference.is_null() {
            return Ok(());
        }

        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        if matches!(location.place, HeapPlace::Young { .. }) && self.mark_place(location.place)? {
            pending.push(reference);
        }

        Ok(())
    }

    /// Clear every collector mark bit in the live heap.
    fn clear_mark_bits(&mut self) -> HeapResult<()> {
        self.young.marked.clear_all();

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            span.clear_marks();
        }

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
    fn is_marked_place(&self, place: HeapPlace) -> HeapResult<bool> {
        match place {
            HeapPlace::Young { first_offset } => {
                let Some((allocation_index, _allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                Ok(self.young.marked.contains(allocation_index))
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
    fn mark_place(&mut self, place: HeapPlace) -> HeapResult<bool> {
        if self.is_marked_place(place)? {
            return Ok(false);
        }

        match place {
            HeapPlace::Young { first_offset } => {
                let Some((allocation_index, _allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                self.young.marked.set(allocation_index);
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

    /// Queue every young reference discovered from remembered mature writes.
    fn enqueue_dirty_young_references(&mut self, pending: &mut HeapTraceQueue) -> HeapResult<()> {
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
        let pages = span.pages.clone();
        let dirty_cards = span.dirty_cards.clone();

        let mut first_error = None;

        // scan each dirty card window against each occupied slot
        dirty_cards.visit_dirty_ranges(|card_start, card_len| {
            if first_error.is_some() {
                return;
            }

            let card_end = card_start.saturating_add(card_len);

            let first_slot = card_start / size_class;
            let last_slot = card_end.saturating_sub(1) / size_class;
            let end_slot = last_slot.saturating_add(1).min(slot_count);

            for slot_index in first_slot..end_slot {
                if !occupied.contains(slot_index) {
                    continue;
                }

                let reference_map = slot_reference_map(
                    &local_reference_bits,
                    &shared_reference_bits,
                    slot_index,
                    size_class,
                    size_class,
                );
                if !reference_map.has_local_reference() {
                    continue;
                }

                let slot_start = size_class.saturating_mul(slot_index);
                let slot_end = slot_start.saturating_add(size_class);
                let overlap_start = card_start.max(slot_start);
                let overlap_end = card_end.min(slot_end);

                if overlap_start >= overlap_end {
                    continue;
                }

                let local_start = overlap_start.saturating_sub(slot_start);
                let local_len = overlap_end.saturating_sub(overlap_start);
                let mut references = Vec::new();
                let result = visit_heap_references_in_reader_range(
                    &reference_map,
                    local_start,
                    local_len,
                    |start, buffer| {
                        self.allocator().fill_bytes_from(
                            &pages,
                            slot_start.saturating_add(start),
                            buffer,
                        )
                    },
                    |reference| references.push(reference),
                );

                if let Err(error) = result {
                    first_error = Some(HeapError::HeapScanFailed {
                        source: ScanSource::Span(span_index),
                        error: Box::new(error),
                    });
                    return;
                }

                for reference in references {
                    if let Err(error) = self.enqueue_young_reference(reference, pending) {
                        first_error = Some(error);
                        return;
                    }
                }
            }
        });

        if let Some(error) = first_error {
            return Err(error);
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
        let pages = allocation.pages.clone();
        let dirty_cards = allocation.dirty_cards.clone();
        let reference_map = allocation.reference_map.clone();

        let mut first_error = None;

        // scan each dirty card window directly against the large allocation
        dirty_cards.visit_dirty_ranges(|card_start, card_len| {
            if first_error.is_some() {
                return;
            }

            let mut references = Vec::new();
            let result = visit_heap_references_in_reader_range(
                &reference_map,
                card_start,
                card_len,
                |start, buffer| self.allocator().fill_bytes_from(&pages, start, buffer),
                |reference| references.push(reference),
            );

            if let Err(error) = result {
                first_error = Some(HeapError::HeapScanFailed {
                    source: ScanSource::LargeAllocation(allocation_id.id()),
                    error: Box::new(error),
                });
                return;
            }

            for reference in references {
                if let Err(error) = self.enqueue_young_reference(reference, pending) {
                    first_error = Some(error);
                    return;
                }
            }
        });

        if let Some(error) = first_error {
            return Err(error);
        }

        // clear only after the scan succeeds
        if let Some(allocation) = self.large_allocation_mut(allocation_id) {
            allocation.dirty_cards.clear();
            allocation.is_dirty_queued = false;
        }

        Ok(())
    }

    /// Build one GC statistics snapshot after one collection pass.
    fn stats_after_collection(&self, freed_allocations: usize, freed_bytes: u64) -> GcStats {
        GcStats {
            freed_allocations,
            live_allocations: self.usage.allocation_count(),
            freed_bytes,
            allocated_bytes: self.usage.allocated_bytes(),
            active_bytes: self.active_bytes(),
        }
    }
}
