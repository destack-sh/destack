use destack_mir::TraceMap;

use super::{
    CardSet, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocation, LargeAllocationId,
    LocalGcPhase, SmallSpan, YoungGcPhase, YoungPlace, YoungRun, YoungRunBits,
};
use crate::allocator::{PageRun, SpanSlot};
use crate::{
    AllocationPlan, Bitmap, HeapAllocationError, HeapError, HeapReference, HeapRepresentationError,
    HeapResult, Payload, SmallAllocationPlan, SmallSpanClass, clear_allocation_reference_bits,
    clear_slot_reference_bits, write_allocation_reference_bits, write_slot_reference_bits,
};

impl HeapSpace {
    /// Return the projected retained-byte delta for one allocation plan.
    pub(crate) fn retained_byte_delta(&self, layout: &AllocationPlan<'_>) -> HeapResult<i64> {
        // reject empty managed heap allocations
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // stay in young space when the payload still fits
        if self.young_fits(layout.byte_len, layout.alignment) {
            return Ok(0);
        }

        // use one traced small span when the payload still fits
        if let Some(small) = layout.class.small() {
            if self.has_available_small_slot(&small) {
                return Ok(0);
            }

            Ok(small.class.span_size_bytes as i64)
        }
        // otherwise allocate one dedicated large allocation
        else {
            Ok(self.round_up_large_allocation_bytes(layout.byte_len) as i64)
        }
    }

    /// Reserve one reference from the active young run cursor.
    #[inline(always)]
    pub(crate) fn reserve_young_run_cursor(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        let reference = self
            .young
            .run_cursor
            .reserve_matching_reference(byte_len, class)?;

        Some(reference)
    }

    /// Reserve one noscan reference from the active young run cursor.
    #[inline(always)]
    pub(crate) fn reserve_young_noscan_run_cursor(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        let reference = self
            .young
            .run_cursor
            .reserve_noscan_reference(byte_len, class)?;

        Some(reference)
    }

    /// Reserve one fixed-size or no-scan payload in young space.
    #[inline(always)]
    fn reserve_young_run_or_noscan_range(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<Option<HeapReference>> {
        if let Some(slot) = self.reserve_young_run(layout)? {
            self.initialize_clean_slot_payload(slot.reference.offset(), payload);

            if !layout.is_noscan {
                self.publish_young_run_allocation(
                    slot,
                    layout.trace_map,
                    payload.byte_len().is_some(),
                )?;
            }
            // blacken noscan allocations created during an active major cycle
            else if self.collector.major_phase != LocalGcPhase::Idle {
                let Some(bits) = self.young.run_bits_mut(slot.run_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.marked.set_in_bounds(slot.slot_index);
            }

            return Ok(Some(slot.reference));
        }

        if !layout.is_noscan {
            return Ok(None);
        }

        let byte_len = layout.byte_len;
        let Some((write_offset, range_index)) =
            self.reserve_young_noscan_range(byte_len, layout.alignment)?
        else {
            return Ok(None);
        };
        let reference = HeapReference::new(write_offset);

        self.initialize_mapped_payload(write_offset, byte_len, payload);

        // blacken noscan allocations created during an active major cycle
        if self.collector.major_phase != LocalGcPhase::Idle {
            self.young.marked.set_in_bounds(range_index);
        }
        self.record_young_allocation(byte_len);

        Ok(Some(reference))
    }

    /// Allocate one fixed-size payload from young space.
    #[inline(always)]
    fn reserve_young_run(
        &mut self,
        layout: &AllocationPlan<'_>,
    ) -> HeapResult<Option<YoungRunSlot>> {
        let byte_len = layout.byte_len;
        let alignment = layout.alignment;

        if alignment > self.young.allocation_alignment_bytes {
            return Ok(None);
        }

        let Some(small) = layout.class.small() else {
            return Ok(None);
        };
        let class = small.class;
        let cache_index = small.cache_index();

        // stay on the active run cursor without consulting metadata
        if self.young.run_cursor.matches(class, byte_len)
            && let Some(reference) = self.young.run_cursor.reserve_reference()
        {
            let run_index = self.young.run_cursor.run_index;
            let Some(run) = self.young.run(run_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_index = (reference.offset() - run.first_offset) / class.size_class;

            return Ok(Some(YoungRunSlot {
                reference,
                run_index,
                slot_index,
            }));
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        // reuse the class bucket run when it still has space
        if let Some(run_index) = self
            .young
            .run_cache
            .get(cache_index)
            .and_then(|run_index| *run_index)
            && let Some(slot) = self.reserve_young_run_slot(byte_len, class, run_index)
        {
            return Ok(Some(slot));
        }

        let Some(run_index) = self.allocate_young_run(byte_len, class, cache_index)? else {
            return Ok(None);
        };

        Ok(self.reserve_young_run_slot(byte_len, class, run_index))
    }

    /// Allocate one fixed-size young run for one small class.
    fn allocate_young_run(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
        cache_index: usize,
    ) -> HeapResult<Option<usize>> {
        let size_class = class.size_class;
        let first_offset = align_up(self.young.next_offset, self.young.page_size_bytes);
        if first_offset >= self.young.capacity_bytes {
            return Ok(None);
        }

        let available_bytes = self.young.capacity_bytes - first_offset;
        let configured_bytes = class.span_size_bytes;
        let run_bytes = configured_bytes.min(available_bytes);
        let run_bytes = run_bytes / self.young.page_size_bytes * self.young.page_size_bytes;
        if run_bytes < size_class {
            return Ok(None);
        }

        let slot_count = run_bytes / size_class;
        let page_start = first_offset / self.young.page_size_bytes;
        let page_count = run_bytes / self.young.page_size_bytes;

        // materialize the whole run before publishing its slots
        self.materialize_young_range(first_offset, run_bytes)?;
        self.clear_mapped_bytes(first_offset, run_bytes);

        // publish the run before handing out its first reference
        let run_index = self.young.runs.len();
        self.young.runs.push(YoungRun {
            first_offset,
            byte_len,
            next_offset: first_offset,
            end_offset: first_offset + run_bytes,
            class,
        });
        self.young.run_bits.push(YoungRunBits {
            freed: Bitmap::with_capacity(slot_count),
            marked: Bitmap::with_capacity(slot_count),
        });

        for page_index in page_start..page_start + page_count {
            self.young.page_runs[page_index] = Some(run_index);
        }

        if self.young.run_cache.len() <= cache_index {
            self.young.run_cache.resize(cache_index + 1, None);
        }
        self.young.run_cache[cache_index] = Some(run_index);
        self.young.next_offset = first_offset + run_bytes;

        Ok(Some(run_index))
    }

    /// Reserve one fixed-size young run slot.
    #[inline(always)]
    fn reserve_young_run_slot(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
        run_index: usize,
    ) -> Option<YoungRunSlot> {
        self.flush_young_run_cursor();
        self.young.activate_run_cursor(byte_len, class, run_index)?;
        let reference = self.young.run_cursor.reserve_reference()?;
        let run = self.young.run(run_index)?;
        let slot_index = (reference.offset() - run.first_offset) / class.size_class;

        Some(YoungRunSlot {
            reference,
            run_index,
            slot_index,
        })
    }

    /// Publish one traced young run allocation to active collectors.
    #[inline(always)]
    fn publish_young_run_allocation(
        &mut self,
        slot: YoungRunSlot,
        trace_map: &TraceMap,
        has_initialized_bytes: bool,
    ) -> HeapResult<()> {
        let reference = slot.reference;

        if trace_map.has_shared_reference() {
            self.collector.track_shared_edge_root(reference);
            if has_initialized_bytes {
                self.queue_shared_edge_root(reference);
            }
        }

        self.publish_major_allocation(
            reference,
            HeapPlace::Young(YoungPlace::Slot(SpanSlot::new(
                slot.run_index,
                slot.slot_index,
            )?)),
            trace_map,
        )
    }

    /// Allocate one managed heap allocation.
    pub fn allocate(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: layout.byte_len,
                    actual,
                },
            ));
        }

        if let Some(reference) = self.reserve_young_payload(layout, payload)? {
            return Ok(reference);
        }

        let has_initialized_bytes = payload.byte_len().is_some();

        self.allocate_mature_layout(layout, payload, has_initialized_bytes)
    }

    /// Reserve one managed heap allocation in young space.
    pub(crate) fn reserve_young_payload(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<Option<HeapReference>> {
        let trace_map = layout.trace_map;
        let tracks_shared_edges = layout.has_shared_reference;
        let has_initialized_bytes = payload.byte_len().is_some();

        if let Some(reference) = self.reserve_young_run_or_noscan_range(layout, payload)? {
            return Ok(Some(reference));
        }

        if let Some(reference) = self.allocate_young(
            layout.byte_len,
            layout.alignment,
            payload,
            trace_map,
            tracks_shared_edges,
        )? {
            // track every live reference whose layout may contain shared edges
            if tracks_shared_edges {
                self.collector.track_shared_edge_root(reference);
            }

            // queue newly published shared edges during an active shared cycle
            if tracks_shared_edges && has_initialized_bytes {
                self.queue_shared_edge_root(reference);
            }

            self.publish_major_allocation(
                reference,
                HeapPlace::Young(YoungPlace::Range {
                    first_offset: reference.offset(),
                }),
                trace_map,
            )?;

            return Ok(Some(reference));
        }

        Ok(None)
    }

    /// Allocate one mature managed heap allocation from one allocation plan.
    pub(crate) fn allocate_mature_layout(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
        has_initialized_bytes: bool,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        let (place, charged_bytes) = self.allocate_mature(layout, payload)?;
        let reference = self.base_reference(place)?;

        // track every live reference whose layout may contain shared edges
        if layout.has_shared_reference {
            self.collector.track_shared_edge_root(reference);
        }

        // queue newly published shared edges during an active shared cycle
        if layout.has_shared_reference && has_initialized_bytes {
            self.queue_shared_edge_root(reference);
        }

        self.publish_major_allocation(reference, place, layout.trace_map)?;
        self.record_mature_allocation(charged_bytes);

        Ok(reference)
    }

    /// Free one heap allocation.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.flush_young_run_cursor();

        let Some(region) = self.resolve_region(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        match region.place {
            // retire one young range until the next young sweep
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((allocation_index, allocation)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::internal("missing young range"));
                };

                self.young.live.clear(allocation_index);
                self.young.marked.clear(allocation_index);
                clear_allocation_reference_bits(
                    &mut self.young.local_reference_bits,
                    &mut self.young.shared_reference_bits,
                    allocation.first_offset,
                    allocation.byte_len,
                );

                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(region.byte_len);

                Ok(())
            }

            // retire one young fixed-size slot until the next young sweep
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(bits) = self.young.run_bits_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.freed.set(slot.slot_index());
                bits.marked.clear(slot.slot_index());
                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(region.byte_len);

                Ok(())
            }

            // release one small-span slot
            HeapPlace::Small(slot) => {
                self.release_small_slot(slot)?;
                self.collector.remove_shared_edge_root(reference);
                self.record_mature_free(region.byte_len);

                Ok(())
            }

            // release one allocation in large space and its allocator pages
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::internal("missing large allocation"));
                };

                let first_offset = allocation.first_offset;
                let pages = allocation.pages;

                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::internal("missing large allocation"));
                };

                // retire the live large-allocation slot before releasing its pages
                allocation.retire();
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());

                self.collector.remove_shared_edge_root(reference);

                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;
                self.record_mature_free(region.byte_len);

                Ok(())
            }
        }
    }

    /// Allocate one mature heap place for the given payload.
    fn allocate_mature(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<(HeapPlace, usize)> {
        // reject inconsistent allocation
        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: layout.byte_len,
                    actual,
                },
            ));
        }

        // allocate from one size class span when the payload still fits
        if let Some(small) = layout.class.small() {
            let class = small.class;
            let span_index = self.allocate_small_span(&class)?;
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_index = span.free_cursor;
            let slot = self.initialize_small_slot(
                &class,
                span_index,
                slot_index,
                layout.byte_len,
                layout.trace_map,
                payload,
                true,
            )?;

            Ok((HeapPlace::Small(slot), class.size_class))
        }
        // otherwise allocate one dedicated large allocation
        else {
            let pages = self.allocate_page_run(layout.byte_len)?;
            let allocation_id = self.insert_large_allocation(
                layout.byte_len,
                layout.alignment,
                pages,
                layout.trace_map.clone(),
                true,
            )?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::internal("missing large allocation"));
            };
            let first_offset = allocation.first_offset;

            self.initialize_mapped_payload(first_offset, layout.byte_len, payload);

            Ok((HeapPlace::Large(allocation_id), layout.byte_len))
        }
    }

    /// Release one heap small slot.
    pub(crate) fn release_small_slot(&mut self, slot: SpanSlot) -> HeapResult<()> {
        let mut requeue_class = None;
        let pages = {
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::internal("missing span"));
            };

            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;

            if !span.occupied.contains(slot_index) {
                return Err(HeapError::internal("missing small slot"));
            }
            if span.occupied_count == 0 {
                return Err(HeapError::internal("missing small slot"));
            }

            span.occupied.clear(slot_index);
            span.marked.clear(slot_index);
            let size_class = span.class.size_class;
            let local_reference_bits = &mut span.local_reference_bits;
            let shared_reference_bits = &mut span.shared_reference_bits;
            clear_slot_reference_bits(
                local_reference_bits,
                shared_reference_bits,
                slot_index,
                size_class,
            );
            span.occupied_count -= 1;
            span.free_cursor = span.free_cursor.min(slot_index);

            // release fully empty span pages back into the local cache
            if span.occupied_count == 0 {
                span.free_cursor = 0;
                span.dirty_cards.clear();
                span.is_dirty_queued = false;

                let pages = span.pages;
                let first_offset = span.first_offset;
                span.pages = PageRun::empty();

                Some((first_offset, pages))
            }
            // otherwise requeue the span if it was full before the free
            else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;
                if should_requeue {
                    requeue_class = Some(span.class);
                }

                None
            }
        };

        if let Some(class) = requeue_class {
            self.small
                .partial_spans
                .entry(class)
                .or_default()
                .push(slot.span_index());
        }

        if let Some((first_offset, pages)) = pages {
            self.unmap_page_run(first_offset, &pages);
            self.release_page_run(pages)?;
        }

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, small: &SmallAllocationPlan) -> bool {
        self.small
            .partial_spans
            .get(&small.class)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Report whether one byte length still fits the young-space tail.
    #[inline(always)]
    fn young_fits(&self, byte_len: usize, alignment: usize) -> bool {
        if self.collector.young_phase != YoungGcPhase::Idle {
            return false;
        }

        if alignment > self.young.allocation_alignment_bytes {
            return false;
        }

        if byte_len > self.max_young_allocation_bytes {
            return false;
        }

        let write_offset = align_up(
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
        );
        if write_offset > self.young.capacity_bytes {
            return false;
        }

        byte_len <= self.young.capacity_bytes - write_offset
    }

    /// Return the page-rounded retained bytes for one heap large allocation.
    fn round_up_large_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.allocator.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }

    /// Insert one large allocation record.
    pub(crate) fn insert_large_allocation(
        &mut self,
        byte_len: usize,
        alignment: usize,
        pages: PageRun,
        trace_map: TraceMap,
        remember: bool,
    ) -> HeapResult<LargeAllocationId> {
        // reuse one freed large allocation id when possible
        let (allocation_id, reused_allocation_id) =
            if let Some(allocation_id) = self.large.free_large_allocation_ids.pop() {
                (allocation_id, true)
            }
            // otherwise allocate from the unused tail
            else {
                let allocation_id = self.large.next_unused_large_allocation_id;
                let next_allocation_id = self.large.next_unused_large_allocation_id + 1;

                self.large.next_unused_large_allocation_id = next_allocation_id;
                (allocation_id, false)
            };

        if allocation_id == 0 {
            if reused_allocation_id {
                self.large.free_large_allocation_ids.push(allocation_id);
            }

            self.release_page_run(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeAllocationId { id: allocation_id },
            ));
        }

        let allocation_id = LargeAllocationId::new(allocation_id);
        let index = allocation_id.index()?;

        let first_offset = self.reserve_space_range_aligned(
            pages.len() * self.allocator.page_size_bytes(),
            alignment,
        )?;

        // materialize the full large allocation before publishing it
        self.mapping
            .materialize(first_offset, pages.len() * self.allocator.page_size_bytes())?;

        self.map_page_run(first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::Large {
                allocation_id,
                logical_page_index,
            }
        });

        // materialize the allocation record
        let allocation = LargeAllocation {
            is_live: true,
            first_offset,
            byte_len,
            pages,
            trace_map,
            mark_epoch: 0,
            dirty_cards: CardSet::with_len(byte_len),
            is_dirty_queued: false,
        };

        if let Err(error) = self.large.allocations.set_or_push(index, allocation) {
            if reused_allocation_id {
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
            }

            self.unmap_page_run(first_offset, &pages);
            self.release_page_run(pages)?;

            return Err(error);
        }

        // remember new mature allocations conservatively
        if remember {
            self.mark_large_allocation_dirty(allocation_id, 0, byte_len)?;
        }

        Ok(allocation_id)
    }

    /// Allocate one mature payload copied out of young space.
    pub(crate) fn allocate_promoted_payload(
        &mut self,
        byte_len: usize,
        trace_map: &TraceMap,
        bytes: &[u8],
    ) -> HeapResult<HeapPlace> {
        let place = if !trace_map.has_reference()
            && let Some(class_index) = self.small.size_classes.class_index_for(byte_len)
        {
            let size_class = self.small.size_classes.classes[class_index];
            let class = SmallSpanClass {
                size_class: size_class.bytes,
                span_size_bytes: size_class
                    .span_size_bytes(self.allocator.page_size_bytes(), self.small.span_size_bytes)
                    .max(self.small.span_size_bytes),
                trace_id: None,
                is_noscan: true,
            };
            let span_index = self.allocate_small_span(&class)?;
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_index = span.free_cursor;
            let slot = self.initialize_small_slot(
                &class,
                span_index,
                slot_index,
                byte_len,
                trace_map,
                Payload::Bytes(bytes),
                false,
            )?;

            self.record_mature_allocation(class.size_class);

            HeapPlace::Small(slot)
        } else {
            let pages = self.allocate_page_run(byte_len)?;
            let allocation_id = self.insert_large_allocation(
                byte_len,
                self.allocator.page_size_bytes(),
                pages,
                trace_map.clone(),
                false,
            )?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::internal("missing large allocation"));
            };

            self.initialize_mapped_payload(
                allocation.first_offset,
                byte_len,
                Payload::Bytes(bytes),
            );
            self.record_mature_allocation(byte_len);

            HeapPlace::Large(allocation_id)
        };

        Ok(place)
    }

    /// Allocate at the young-space bump cursor when the request fits.
    fn allocate_young(
        &mut self,
        byte_len: usize,
        alignment: usize,
        payload: Payload<'_>,
        trace_map: &TraceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(write_offset) =
            self.reserve_young_range(byte_len, alignment, trace_map, tracks_shared_edges)?
        else {
            return Ok(None);
        };

        self.initialize_mapped_payload(write_offset, byte_len, payload);

        Ok(Some(HeapReference::new(write_offset)))
    }

    /// Materialize the young-space pages needed by one allocation.
    #[inline(always)]
    fn materialize_young_range(&mut self, offset: usize, byte_len: usize) -> HeapResult<()> {
        // stay on the already mapped prefix
        let end = offset + byte_len;
        if end <= self.young.mapped_until {
            return Ok(());
        }

        // amortize native mapping across several allocator refills
        let materialize_bytes = self
            .young
            .capacity_bytes
            .min(self.small.span_size_bytes * 8)
            .max(byte_len);
        let materialize_end = (offset + materialize_bytes).min(self.young.capacity_bytes);

        // round to native page frames
        let frame_bytes = self.mapping.frame_bytes();
        let frame_start = offset / frame_bytes * frame_bytes;
        let frame_end = materialize_end.div_ceil(frame_bytes) * frame_bytes;

        // publish the new materialized prefix
        self.mapping
            .materialize(frame_start, frame_end - frame_start)?;
        self.young.mapped_until = frame_end;

        Ok(())
    }

    /// Reserve one aligned young-space byte range.
    #[inline(always)]
    fn reserve_young_range(
        &mut self,
        byte_len: usize,
        alignment: usize,
        trace_map: &TraceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<usize>> {
        if alignment > self.young.allocation_alignment_bytes {
            return Ok(None);
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        if trace_map.has_tagged_reference() {
            return Ok(None);
        }

        // reject allocations that do not fit the young-space tail
        let write_offset = align_up(
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
        );
        if write_offset > self.young.capacity_bytes {
            return Ok(None);
        }
        if byte_len > self.young.capacity_bytes - write_offset {
            return Ok(None);
        }

        let end_offset = write_offset + byte_len;
        // materialize the allocation range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install the metadata before exposing the address
        let _range_index = self.young.push_range(write_offset, byte_len);
        if trace_map.has_local_reference() || tracks_shared_edges {
            write_allocation_reference_bits(
                trace_map,
                &mut self.young.local_reference_bits,
                &mut self.young.shared_reference_bits,
                write_offset,
                byte_len,
            );
        }

        // advance the young-space tail after installing the allocation
        self.young.next_offset = end_offset;
        self.record_young_allocation(byte_len);

        Ok(Some(write_offset))
    }

    /// Reserve one aligned no-scan young-space byte range.
    #[inline(always)]
    fn reserve_young_noscan_range(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<Option<(usize, usize)>> {
        if alignment > self.young.allocation_alignment_bytes {
            return Ok(None);
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        let write_offset = align_up(
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
        );
        if write_offset > self.young.capacity_bytes {
            return Ok(None);
        }
        if byte_len > self.young.capacity_bytes - write_offset {
            return Ok(None);
        }

        let end_offset = write_offset + byte_len;
        // materialize the allocation range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install exact no-scan metadata
        let range_index = self.young.push_range(write_offset, byte_len);

        self.young.next_offset = end_offset;

        Ok(Some((write_offset, range_index)))
    }

    /// Allocate or reuse one non-full heap span for the given size class.
    fn allocate_small_span(&mut self, class: &SmallSpanClass) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self.small.partial_spans.entry(*class).or_default().pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let first_offset = span.first_offset;
                    let pages = self.allocate_page_run(class.span_size_bytes)?;

                    self.map_page_run(first_offset, &pages, |logical_page_index| {
                        HeapPageMapEntry::Small {
                            span_index,
                            logical_page_index,
                        }
                    });

                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.unmap_page_run(first_offset, &pages);
                        self.release_page_run(pages)?;

                        return Err(HeapError::internal("missing span"));
                    };

                    // materialize the full span before handing out slots
                    self.mapping
                        .materialize(first_offset, class.span_size_bytes)?;

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (class.span_size_bytes / class.size_class).max(1);
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let dirty_card_bytes = slot_count * class.size_class;
        let pages = self.allocate_page_run(class.span_size_bytes)?;
        let first_offset = self.reserve_space_range(class.span_size_bytes)?;

        // materialize the full span before handing out slots
        self.mapping
            .materialize(first_offset, class.span_size_bytes)?;

        let span = SmallSpan {
            first_offset,
            class: *class,
            slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: Bitmap::with_capacity(slot_count),
            local_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            marked: Bitmap::with_capacity(slot_count),
            mark_epoch: 0,
            pages,
            dirty_cards: CardSet::with_len(dirty_card_bytes),
            is_dirty_queued: false,
        };
        let span_index = self.small.spans.len();
        self.map_page_run(first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::Small {
                span_index,
                logical_page_index,
            }
        });

        self.small.spans.push(span);

        Ok(span_index)
    }

    /// Initialize one reserved heap small-span payload.
    fn initialize_small_slot(
        &mut self,
        class: &SmallSpanClass,
        span_index: usize,
        slot_index: usize,
        byte_len: usize,
        trace_map: &TraceMap,
        init: Payload<'_>,
        remember: bool,
    ) -> HeapResult<SpanSlot> {
        let span = self
            .small
            .spans
            .get(span_index)
            .ok_or(HeapError::internal("missing span"))?;
        let slot_offset = span.class.size_class * slot_index;
        let mapping_offset = span.first_offset + slot_offset;

        self.initialize_mapped_payload(mapping_offset, class.size_class, init);

        let span = self
            .small
            .spans
            .get_mut(span_index)
            .ok_or(HeapError::internal("missing span"))?;

        // publish direct trace metadata when the class has no table id
        if span.class.trace_id.is_none() {
            let size_class = span.class.size_class;
            let local_reference_bits = &mut span.local_reference_bits;
            let shared_reference_bits = &mut span.shared_reference_bits;
            write_slot_reference_bits(
                trace_map,
                local_reference_bits,
                shared_reference_bits,
                slot_index,
                size_class,
            );
        }

        span.occupied.set(slot_index);
        span.marked.clear(slot_index);
        span.occupied_count += 1;
        span.free_cursor = span
            .occupied
            .first_clear_from(slot_index)
            .unwrap_or(span.slot_count);
        let should_requeue = span.occupied_count < span.slot_count;

        // requeue the reserved span when it still has capacity
        if should_requeue {
            self.small
                .partial_spans
                .entry(*class)
                .or_default()
                .push(span_index);
        }

        let slot = SpanSlot::new(span_index, slot_index)?;

        // remember new mature allocations conservatively
        if remember {
            self.mark_span_slot_dirty(span_index, slot_index, 0, byte_len, trace_map)?;
        }

        Ok(slot)
    }

    /// Initialize one mapped payload range.
    #[inline(always)]
    pub(super) fn initialize_mapped_payload(
        &self,
        offset: usize,
        clear_byte_len: usize,
        payload: Payload<'_>,
    ) {
        match payload {
            Payload::Bytes(bytes) if bytes.len() < clear_byte_len => {
                self.clear_mapped_bytes(offset, clear_byte_len);
                self.write_mapped_bytes(offset, bytes);
            }
            Payload::Bytes(bytes) => self.write_mapped_bytes(offset, bytes),
            Payload::Zeroed => self.clear_mapped_bytes(offset, clear_byte_len),
            Payload::Uninit => {}
        }
    }

    /// Initialize one already-zeroed small slot.
    #[inline(always)]
    pub(super) fn initialize_clean_slot_payload(&self, offset: usize, payload: Payload<'_>) {
        if let Payload::Bytes(bytes) = payload {
            self.write_mapped_bytes(offset, bytes);
        }
    }

    /// Write bytes into one mapped payload range.
    #[inline(always)]
    pub(super) fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        // SAFETY: allocation paths materialize the destination before publishing it
        unsafe {
            self.mapping.write_mapped_bytes(offset, bytes);
        }
    }

    /// Clear one mapped payload range.
    #[inline(always)]
    pub(super) fn clear_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let address = self.mapping.base_address() + offset;

        // SAFETY: allocation paths materialize the destination before publishing it
        unsafe {
            std::ptr::write_bytes(address as *mut u8, 0, byte_len);
        }
    }
}

/// One reserved fixed-size young slot.
#[derive(Debug, Clone, Copy)]
struct YoungRunSlot {
    /// The allocated heap reference.
    reference: HeapReference,
    /// The owning young run index.
    run_index: usize,
    /// The slot index inside the owning young run.
    slot_index: usize,
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
