use destack_mir::ReferenceMap;

use super::{
    CardSet, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocation, LargeAllocationId, SmallSpan,
    YoungRange,
};
use crate::allocator::{PageRun, SpanSlot};
use crate::{
    AllocationLayout, Bitmap, HeapError, HeapReference, HeapResult, Payload, SmallSpanClass,
    clear_allocation_reference_bits, clear_slot_reference_bits, write_allocation_reference_bits,
    write_slot_reference_bits,
};

impl HeapSpace {
    /// Return the projected retained-byte delta for one managed allocation.
    pub(crate) fn retained_byte_delta(&self, layout: AllocationLayout<'_>) -> HeapResult<i64> {
        // reject empty managed heap allocations
        if layout.byte_len == 0 {
            return Err(HeapError::ZeroSizeAllocation);
        }

        // stay in young space when the payload still fits
        if self.young_fits(layout.byte_len) {
            return Ok(0);
        }

        // use one traced small span when the payload still fits
        if let Some(class) = self.small_span_class(layout.byte_len, layout.reference_map) {
            if self.has_available_small_slot(&class)? {
                return Ok(0);
            }

            Ok(class.span_bytes as i64)
        }
        // otherwise fall back to one dedicated large allocation
        else {
            Ok(self.round_up_large_allocation_bytes(layout.byte_len) as i64)
        }
    }

    /// Allocate one managed heap allocation.
    pub fn allocate(
        &mut self,
        layout: AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        if layout.byte_len == 0 {
            return Err(HeapError::ZeroSizeAllocation);
        }

        let reference_map = layout.reference_map;
        let tracks_shared_edges = reference_map.has_shared_reference();
        let has_initialized_bytes = payload.byte_len().is_some();
        let (place, charged_bytes) =
            self.allocate_place(layout.byte_len, reference_map, payload)?;
        let reference = self.base_reference(place)?;

        self.usage.allocate(charged_bytes);

        // track every live reference whose layout may contain shared edges
        if tracks_shared_edges {
            self.track_shared_edge_root(reference)?;
        }

        // queue newly published shared edges during an active shared cycle
        if has_initialized_bytes {
            self.queue_shared_reference(reference)?;
        }

        self.publish_major_allocation(reference, place)?;

        Ok(reference)
    }

    /// Free one heap allocation.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<bool> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let freed_bytes = location.byte_len as u64;
        self.usage.check_free(freed_bytes);

        match location.place {
            // retire one young range until the next scavenge
            HeapPlace::Young { first_offset } => {
                let Some((allocation_index, allocation)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                let allocation = allocation.clone();

                self.young.live.clear(allocation_index);
                self.young.marked.clear(allocation_index);
                clear_allocation_reference_bits(
                    &mut self.young.local_reference_bits,
                    &mut self.young.shared_reference_bits,
                    allocation.first_offset,
                    allocation.byte_len,
                );

                self.usage.free(freed_bytes);
                self.remove_shared_edge_root(reference)?;

                Ok(true)
            }

            // release one small-span slot
            HeapPlace::Small(slot) => {
                self.release_small_slot(slot)?;
                self.usage.free(freed_bytes);
                self.remove_shared_edge_root(reference)?;

                Ok(true)
            }

            // release one allocation in large space and its allocator pages
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                let first_offset = allocation.first_offset;
                let pages = allocation.pages;

                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                // retire the live large-allocation slot before releasing its pages
                allocation.retire();
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());

                self.usage.free(freed_bytes);
                self.remove_shared_edge_root(reference)?;

                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                Ok(true)
            }
        }
    }

    /// Allocate one heap place for the given payload.
    fn allocate_place(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
        payload: Payload<'_>,
    ) -> HeapResult<(HeapPlace, usize)> {
        // reject inconsistent allocation
        if let Some(actual) = payload.byte_len()
            && actual != byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: byte_len,
                actual,
            });
        }

        // try the young path first
        if let Some(first_offset) = self.allocate_young(byte_len, payload, reference_map)? {
            Ok((HeapPlace::Young { first_offset }, byte_len))
        }
        // otherwise allocate from one size class span when the payload still fits
        else if let Some(class) = self.small_span_class(byte_len, reference_map) {
            let span_index = self.allocate_small_span(&class)?;
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let slot_index = span.free_cursor;
            let slot = self.initialize_small_slot(
                &class,
                span_index,
                slot_index,
                byte_len,
                reference_map,
                payload,
                true,
            )?;

            Ok((HeapPlace::Small(slot), class.size_class))
        }
        // otherwise allocate one dedicated large allocation
        else {
            let pages = self.allocate_large_pages(byte_len)?;
            let allocation_id =
                self.insert_large_allocation(byte_len, pages, reference_map.clone(), true)?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                });
            };
            let first_offset = allocation.first_offset;

            // initialize bytes before returning the allocation reference
            let initialize = match payload {
                Payload::Bytes(bytes) => self.mapping.write(first_offset, bytes),
                Payload::Zeroed => self.mapping.zero(first_offset, byte_len),
            };
            if let Err(error) = initialize {
                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    self.unmap_page_run(first_offset, &pages);
                    self.release_page_run(pages)?;

                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                allocation.retire();
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                return Err(error);
            }

            Ok((HeapPlace::Large(allocation_id), byte_len))
        }
    }

    /// Release one heap small slot.
    pub(crate) fn release_small_slot(&mut self, slot: SpanSlot) -> HeapResult<()> {
        let mut requeue_class = None;
        let pages = {
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                });
            };

            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;

            if !span.occupied.contains(slot_index) {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
            }
            if span.occupied_count == 0 {
                return Err(HeapError::MissingSmallSlot {
                    span_index: slot.span_index(),
                    slot_index,
                });
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
                    requeue_class = Some(span.class.clone());
                }

                None
            }
        };

        if let Some(class) = requeue_class {
            let bucket_index = self.small_span_bucket(&class)?;
            self.small.partial_spans[bucket_index].push(slot.span_index());
        }

        if let Some((first_offset, pages)) = pages {
            self.unmap_page_run(first_offset, &pages);
            self.release_page_run(pages)?;
        }

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class: &SmallSpanClass) -> HeapResult<bool> {
        let bucket_index = self.small_span_bucket(class)?;

        Ok(self
            .small
            .partial_spans
            .get(bucket_index)
            .is_some_and(|spans| !spans.is_empty()))
    }

    /// Report whether one byte length still fits the young-space tail.
    fn young_fits(&self, byte_len: usize) -> bool {
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

    /// Return one small-span size and scan class for the given payload when it fits.
    fn small_span_class(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> Option<SmallSpanClass> {
        let class_index = self.small.size_classes.class_index_for(byte_len)?;
        let size_class = self.small.size_classes.classes[class_index];

        Some(SmallSpanClass {
            size_class: size_class.bytes,
            span_bytes: size_class.span_bytes(self.allocator.page_bytes(), self.small.span_bytes),
            is_noscan: !reference_map.has_reference(),
        })
    }

    /// Return the page-rounded retained bytes for one heap large allocation.
    fn round_up_large_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Insert one large allocation record.
    pub(crate) fn insert_large_allocation(
        &mut self,
        len: usize,
        pages: PageRun,
        reference_map: ReferenceMap,
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

            return Err(HeapError::InvalidLargeAllocationId { id: allocation_id });
        }

        let allocation_id = LargeAllocationId::new(allocation_id);
        let index = allocation_id.index()?;

        let first_offset = self.reserve_space_range(pages.len() * self.allocator.page_bytes())?;

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
            len,
            pages,
            reference_map,
            is_marked: false,
            dirty_cards: CardSet::with_len(len),
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
            self.mark_large_allocation_dirty(allocation_id, 0, len)?;
        }

        Ok(allocation_id)
    }

    /// Allocate at the young-space bump cursor when the request fits.
    fn allocate_young(
        &mut self,
        byte_len: usize,
        payload: Payload<'_>,
        reference_map: &ReferenceMap,
    ) -> HeapResult<Option<usize>> {
        let Some(write_offset) = self.reserve_young_range(byte_len, reference_map)? else {
            return Ok(None);
        };

        // initialize the live mapping
        match payload {
            Payload::Bytes(bytes) => self.mapping.write(write_offset, bytes)?,
            Payload::Zeroed => self.mapping.zero(write_offset, byte_len)?,
        }

        Ok(Some(write_offset))
    }

    /// Reserve one aligned young-space byte range.
    fn reserve_young_range(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> HeapResult<Option<usize>> {
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

        // append one young range
        let range_index = self.young.ranges.len();
        let range = YoungRange {
            byte_len,
            first_offset: write_offset,
        };

        // install the metadata before exposing the address
        self.young.ranges.push(range);
        self.young.live.ensure_capacity(range_index + 1);
        self.young.marked.ensure_capacity(range_index + 1);
        self.young.live.set(range_index);
        self.young.marked.clear(range_index);
        write_allocation_reference_bits(
            reference_map,
            &mut self.young.local_reference_bits,
            &mut self.young.shared_reference_bits,
            write_offset,
            byte_len,
        )?;

        // advance the young-space tail after installing the allocation
        self.young.next_offset = end_offset;

        Ok(Some(write_offset))
    }

    /// Allocate one copied small payload from explicit bytes.
    pub(crate) fn allocate_small_payload_from_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        remember: bool,
    ) -> HeapResult<Option<SpanSlot>> {
        let byte_len = bytes.len();
        let Some((class, span_index, slot_index)) =
            self.reserve_small_payload(byte_len, reference_map)?
        else {
            return Ok(None);
        };

        self.initialize_small_slot(
            &class,
            span_index,
            slot_index,
            byte_len,
            reference_map,
            Payload::Bytes(bytes),
            remember,
        )
        .map(Some)
    }

    /// Allocate one dedicated large-allocation page run.
    fn allocate_large_pages(&mut self, byte_len: usize) -> HeapResult<PageRun> {
        self.allocate_page_run_zeroed(byte_len)
    }

    /// Reserve one small-span payload location for the given runtime facts.
    fn reserve_small_payload(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> HeapResult<Option<(SmallSpanClass, usize, usize)>> {
        // resolve the matching size class first
        let Some(class) = self.small_span_class(byte_len, reference_map) else {
            return Ok(None);
        };
        let span_index = self.allocate_small_span(&class)?;
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_index = span.free_cursor;

        Ok(Some((class, span_index, slot_index)))
    }

    /// Allocate or reuse one non-full heap span for the given size class.
    fn allocate_small_span(&mut self, class: &SmallSpanClass) -> HeapResult<usize> {
        let bucket_index = self.small_span_bucket(class)?;

        // reuse one non-full span when possible
        while let Some(span_index) = self.small.partial_spans[bucket_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let first_offset = span.first_offset;
                    let pages = self.allocate_page_run_zeroed(class.span_bytes)?;

                    self.map_page_run(first_offset, &pages, |logical_page_index| {
                        HeapPageMapEntry::Small {
                            span_index,
                            logical_page_index,
                        }
                    });

                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.unmap_page_run(first_offset, &pages);
                        self.release_page_run(pages)?;

                        return Err(HeapError::MissingSpan { span_index });
                    };

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (class.span_bytes / class.size_class).max(1);
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let dirty_card_bytes = slot_count * class.size_class;
        let pages = self.allocate_page_run_zeroed(class.span_bytes)?;
        let first_offset = self.reserve_space_range(class.span_bytes)?;
        let span = SmallSpan {
            first_offset,
            class: class.clone(),
            slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: Bitmap::with_capacity(slot_count),
            local_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            marked: Bitmap::with_capacity(slot_count),
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
        reference_map: &ReferenceMap,
        init: Payload<'_>,
        remember: bool,
    ) -> HeapResult<SpanSlot> {
        let span = self
            .small
            .spans
            .get(span_index)
            .ok_or(HeapError::MissingSpan { span_index })?;
        let slot_offset = span.class.size_class * slot_index;
        let mapping_offset = span.first_offset + slot_offset;

        // clear the full slot before publishing caller bytes
        match init {
            Payload::Bytes(bytes) if bytes.len() < class.size_class => {
                self.mapping.zero(mapping_offset, class.size_class)?;
                self.mapping.write(mapping_offset, bytes)?;
            }
            Payload::Bytes(bytes) => self.mapping.write(mapping_offset, bytes)?,
            Payload::Zeroed => self.mapping.zero(mapping_offset, class.size_class)?,
        }

        let span = self
            .small
            .spans
            .get_mut(span_index)
            .ok_or(HeapError::MissingSpan { span_index })?;

        // publish the initialized slot metadata
        let size_class = span.class.size_class;
        let local_reference_bits = &mut span.local_reference_bits;
        let shared_reference_bits = &mut span.shared_reference_bits;
        write_slot_reference_bits(
            reference_map,
            local_reference_bits,
            shared_reference_bits,
            slot_index,
            size_class,
        )?;

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
            let bucket_index = self.small_span_bucket(class)?;
            self.small.partial_spans[bucket_index].push(span_index);
        }

        let slot = SpanSlot::new(span_index, slot_index)?;

        // remember new mature allocations conservatively
        if remember {
            self.mark_span_slot_dirty(span_index, slot_index, 0, byte_len)?;
        }

        Ok(slot)
    }
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
