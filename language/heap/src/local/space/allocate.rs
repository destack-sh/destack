use std::ptr::write_bytes;

use destack_mir::ReferenceMap;

use super::{
    CardSet, HeapPageMapEntry, HeapPlace, HeapSpace, LargeAllocation, LargeAllocationId,
    LocalGcPhase, SmallSpan, YoungPlace, YoungRun, YoungRunBits,
};
use crate::allocator::{PageRun, SpanSlot};
use crate::{
    AllocationLayout, Bitmap, HeapError, HeapReference, HeapResult, Payload, SmallAllocationLayout,
    SmallSpanClass, clear_allocation_reference_bits, clear_slot_reference_bits,
    write_allocation_reference_bits, write_slot_reference_bits,
};

impl HeapSpace {
    /// Return the projected retained-byte delta for one allocation layout.
    pub(crate) fn retained_byte_delta(&self, layout: &AllocationLayout<'_>) -> HeapResult<i64> {
        // reject empty managed heap allocations
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
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

            Ok(small.class.span_bytes as i64)
        }
        // otherwise allocate one dedicated large allocation
        else {
            Ok(self.round_up_large_allocation_bytes(layout.byte_len) as i64)
        }
    }

    /// Report whether one allocation layout fits the young-space tail.
    #[inline(always)]
    pub(crate) fn layout_fits_young(&self, layout: &AllocationLayout<'_>) -> bool {
        if layout.is_empty() {
            return false;
        }

        self.young_fits(layout.byte_len, layout.alignment)
    }

    /// Try to allocate one zeroed allocation in young space.
    #[inline(always)]
    pub(crate) fn try_allocate_young_zeroed(
        &mut self,
        layout: &AllocationLayout<'_>,
    ) -> HeapResult<Option<HeapReference>> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if layout.is_noscan
            && let Some(reference) = self.try_allocate_young_noscan_zeroed(layout)?
        {
            return Ok(Some(reference));
        }

        let reference_map = layout.reference_map;
        let tracks_shared_edges = layout.has_shared_reference;
        let Some(reference) = self.allocate_young_zeroed(
            layout.byte_len,
            layout.alignment,
            reference_map,
            tracks_shared_edges,
        )?
        else {
            return Ok(None);
        };

        // track every live reference whose layout may contain shared edges
        if tracks_shared_edges {
            self.track_shared_edge_root(reference)?;
        }

        self.publish_major_allocation(
            reference,
            HeapPlace::Young(YoungPlace::Range {
                first_offset: reference.offset(),
            }),
        )?;

        Ok(Some(reference))
    }

    /// Reserve one reference from the active young run cursor.
    #[inline(always)]
    pub(crate) fn reserve_young_run_cursor(&mut self, slot_bytes: usize) -> Option<HeapReference> {
        self.young.run_cursor.reserve_matching_reference(slot_bytes)
    }

    /// Try to allocate one byte-initialized allocation in young space.
    #[inline(always)]
    pub(crate) fn try_allocate_young_bytes(
        &mut self,
        layout: &AllocationLayout<'_>,
        bytes: &[u8],
    ) -> HeapResult<Option<HeapReference>> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        if layout.is_noscan
            && let Some(reference) = self.try_allocate_young_noscan_bytes(layout, bytes)?
        {
            return Ok(Some(reference));
        }

        let reference_map = layout.reference_map;
        let tracks_shared_edges = layout.has_shared_reference;
        let Some(reference) = self.allocate_young_range_bytes(
            layout.byte_len,
            layout.alignment,
            bytes,
            reference_map,
            tracks_shared_edges,
        )?
        else {
            return Ok(None);
        };

        // track every live reference whose layout may contain shared edges
        if tracks_shared_edges {
            self.track_shared_edge_root(reference)?;
            self.queue_shared_reference(reference)?;
        }

        self.publish_major_allocation(
            reference,
            HeapPlace::Young(YoungPlace::Range {
                first_offset: reference.offset(),
            }),
        )?;

        Ok(Some(reference))
    }

    /// Try to allocate one zeroed no-scan allocation in young space.
    #[inline(always)]
    fn try_allocate_young_noscan_zeroed(
        &mut self,
        layout: &AllocationLayout<'_>,
    ) -> HeapResult<Option<HeapReference>> {
        let byte_len = layout.byte_len;

        if let Some(slot) = self.reserve_young_run(layout)? {
            return Ok(Some(slot.reference));
        }

        let Some((write_offset, start_index)) =
            self.reserve_young_noscan_range(byte_len, layout.alignment)?
        else {
            return Ok(None);
        };
        let reference = HeapReference::new(write_offset);

        // objects allocated during marking start black
        if self.major_phase == LocalGcPhase::Mark {
            self.young.marked.set_in_bounds(start_index);
        }

        Ok(Some(reference))
    }

    /// Allocate one fixed-size no-scan payload from young space.
    #[inline(always)]
    fn reserve_young_run(
        &mut self,
        layout: &AllocationLayout<'_>,
    ) -> HeapResult<Option<YoungRunSlot>> {
        let byte_len = layout.byte_len;
        let alignment = layout.alignment;

        if alignment > self.young.allocation_alignment_bytes {
            return Ok(None);
        }

        // stay on the active run cursor without consulting metadata
        if self.young.run_cursor.matches(byte_len)
            && let Some(reference) = self.young.run_cursor.reserve_reference()
        {
            return Ok(Some(YoungRunSlot { reference }));
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        let Some(small) = layout.class.small() else {
            return Ok(None);
        };
        let bucket_index = small.bucket_index;
        let class_index = small.class_index;
        let minimum_byte_len = small.minimum_byte_len;
        let size_class = small.class.size_class;

        // reuse the class bucket run when it still has space
        if let Some(run_index) = self.young.run_buckets.get(bucket_index).copied().flatten()
            && let Some(slot) = self.reserve_young_run_slot(minimum_byte_len, size_class, run_index)
        {
            return Ok(Some(slot));
        }

        let Some(run_index) = self.allocate_young_run(bucket_index, class_index)? else {
            return Ok(None);
        };

        Ok(self.reserve_young_run_slot(minimum_byte_len, size_class, run_index))
    }

    /// Allocate one fixed-size young run for one no-scan size class.
    fn allocate_young_run(
        &mut self,
        bucket_index: usize,
        class_index: usize,
    ) -> HeapResult<Option<usize>> {
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let first_offset = align_up(self.young.next_offset, self.young.page_bytes);
        if first_offset >= self.young.capacity_bytes {
            return Ok(None);
        }

        let available_bytes = self.young.capacity_bytes - first_offset;
        let configured_bytes = self.small.size_classes.classes[class_index]
            .span_bytes(self.young.page_bytes, self.small.span_bytes)
            .max(self.small.span_bytes);
        let run_bytes = configured_bytes.min(available_bytes);
        let run_bytes = run_bytes / self.young.page_bytes * self.young.page_bytes;
        if run_bytes < size_class {
            return Ok(None);
        }

        let slot_count = run_bytes / size_class;
        let page_start = first_offset / self.young.page_bytes;
        let page_count = run_bytes / self.young.page_bytes;
        let class = SmallSpanClass {
            size_class,
            span_bytes: run_bytes,
            is_noscan: true,
        };

        // materialize the whole run before publishing its slots
        self.materialize_young_range(first_offset, run_bytes)?;

        // publish the run before handing out its first reference
        let run_index = self.young.runs.len();
        self.young.runs.push(YoungRun {
            first_offset,
            next_offset: first_offset,
            end_offset: first_offset + run_bytes,
            size_class: class.size_class,
            span_bytes: class.span_bytes,
            slot_count,
        });
        self.young.run_bits.push(YoungRunBits {
            freed: Bitmap::with_capacity(slot_count),
            marked: Bitmap::with_capacity(slot_count),
        });
        self.young
            .run_forwarded
            .push(vec![0; slot_count].into_boxed_slice());

        for page_index in page_start..page_start + page_count {
            self.young.page_runs[page_index] = Some(run_index);
        }

        self.young.run_buckets[bucket_index] = Some(run_index);
        self.young.next_offset = first_offset + run_bytes;

        Ok(Some(run_index))
    }

    /// Reserve one fixed-size young run slot.
    #[inline(always)]
    fn reserve_young_run_slot(
        &mut self,
        minimum_byte_len: usize,
        size_class: usize,
        run_index: usize,
    ) -> Option<YoungRunSlot> {
        self.young
            .activate_run_cursor(minimum_byte_len, size_class, run_index)?;
        let reference = self.young.run_cursor.reserve_reference()?;

        Some(YoungRunSlot { reference })
    }

    /// Try to allocate one byte-initialized no-scan allocation in young space.
    #[inline(always)]
    fn try_allocate_young_noscan_bytes(
        &mut self,
        layout: &AllocationLayout<'_>,
        bytes: &[u8],
    ) -> HeapResult<Option<HeapReference>> {
        let byte_len = layout.byte_len;

        if let Some(slot) = self.reserve_young_run(layout)? {
            let reference = slot.reference;
            unsafe {
                self.mapping.write_mapped_bytes(reference.offset(), bytes);
            }

            return Ok(Some(reference));
        }

        let Some((write_offset, start_index)) =
            self.reserve_young_noscan_range(byte_len, layout.alignment)?
        else {
            return Ok(None);
        };
        let reference = HeapReference::new(write_offset);

        unsafe {
            self.mapping.write_mapped_bytes(write_offset, bytes);
        }

        // objects allocated during marking start black
        if self.major_phase == LocalGcPhase::Mark {
            self.young.marked.set_in_bounds(start_index);
        }

        Ok(Some(reference))
    }

    /// Allocate one managed heap allocation.
    pub fn allocate(
        &mut self,
        layout: &AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        let reference_map = layout.reference_map;
        let tracks_shared_edges = layout.has_shared_reference;
        let has_initialized_bytes = payload.byte_len().is_some();

        if let Some(reference) = self.allocate_young(
            layout.byte_len,
            layout.alignment,
            payload,
            reference_map,
            tracks_shared_edges,
        )? {
            // track every live reference whose layout may contain shared edges
            if tracks_shared_edges {
                self.track_shared_edge_root(reference)?;
            }

            // queue newly published shared edges during an active shared cycle
            if tracks_shared_edges && has_initialized_bytes {
                self.queue_shared_reference(reference)?;
            }

            self.publish_major_allocation(
                reference,
                HeapPlace::Young(YoungPlace::Range {
                    first_offset: reference.offset(),
                }),
            )?;

            return Ok(reference);
        }

        self.allocate_mature_layout(layout, payload, has_initialized_bytes)
    }

    /// Allocate one mature managed heap allocation from one allocation layout.
    pub(crate) fn allocate_mature_layout(
        &mut self,
        layout: &AllocationLayout<'_>,
        payload: Payload<'_>,
        has_initialized_bytes: bool,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::ZeroSizeAllocation);
        }

        let (place, _) = self.allocate_mature(layout, payload)?;
        let reference = self.base_reference(place)?;

        // track every live reference whose layout may contain shared edges
        if layout.has_shared_reference {
            self.track_shared_edge_root(reference)?;
        }

        // queue newly published shared edges during an active shared cycle
        if layout.has_shared_reference && has_initialized_bytes {
            self.queue_shared_reference(reference)?;
        }

        self.publish_major_allocation(reference, place)?;

        Ok(reference)
    }

    /// Free one heap allocation.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        match location.place {
            // retire one young range until the next scavenge
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((allocation_index, allocation)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                self.young.live.clear(allocation_index);
                self.young.marked.clear(allocation_index);
                clear_allocation_reference_bits(
                    &mut self.young.local_reference_bits,
                    &mut self.young.shared_reference_bits,
                    allocation.first_offset,
                    allocation.byte_len,
                );

                self.remove_shared_edge_root(reference)?;

                Ok(())
            }

            // retire one young fixed-size slot until the next scavenge
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let Some(bits) = self.young.run_bits_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                bits.freed.set(slot.slot_index());
                bits.marked.clear(slot.slot_index());
                self.remove_shared_edge_root(reference)?;

                Ok(())
            }

            // release one small-span slot
            HeapPlace::Small(slot) => {
                self.release_small_slot(slot)?;
                self.remove_shared_edge_root(reference)?;

                Ok(())
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

                self.remove_shared_edge_root(reference)?;

                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                Ok(())
            }
        }
    }

    /// Allocate one mature heap place for the given payload.
    fn allocate_mature(
        &mut self,
        layout: &AllocationLayout<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<(HeapPlace, usize)> {
        // reject inconsistent allocation
        if let Some(actual) = payload.byte_len()
            && actual != layout.byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: layout.byte_len,
                actual,
            });
        }

        // allocate from one size class span when the payload still fits
        if let Some(small) = layout.class.small() {
            let class = small.class;
            let span_index = self.allocate_small_span(&class)?;
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let slot_index = span.free_cursor;
            let slot = self.initialize_small_slot(
                &class,
                span_index,
                slot_index,
                layout.byte_len,
                layout.reference_map,
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
                layout.reference_map.clone(),
                true,
            )?;
            let Some(allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                });
            };
            let first_offset = allocation.first_offset;

            // initialize bytes before returning the allocation reference
            match payload {
                Payload::Bytes(bytes) => unsafe {
                    self.mapping.write_mapped_bytes(first_offset, bytes);
                },
                Payload::Zeroed => unsafe {
                    write_bytes(
                        (self.mapping.base_address() + first_offset) as *mut u8,
                        0,
                        layout.byte_len,
                    );
                },
            }

            Ok((HeapPlace::Large(allocation_id), layout.byte_len))
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
                    requeue_class = Some(span.class);
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
    fn has_available_small_slot(&self, small: &SmallAllocationLayout) -> bool {
        let bucket_index = small.bucket_index;

        self.small
            .partial_spans
            .get(bucket_index)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Report whether one byte length still fits the young-space tail.
    #[inline(always)]
    fn young_fits(&self, byte_len: usize, alignment: usize) -> bool {
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

    /// Return one small-span size and scan class for the given payload when it fits.
    fn small_span_class(
        &self,
        byte_len: usize,
        alignment: usize,
        reference_map: &ReferenceMap,
    ) -> Option<SmallSpanClass> {
        let class_index = self
            .small
            .size_classes
            .class_index_for_layout(byte_len, alignment)?;
        let size_class = self.small.size_classes.classes[class_index];

        Some(SmallSpanClass {
            size_class: size_class.bytes,
            span_bytes: size_class
                .span_bytes(self.allocator.page_bytes(), self.small.span_bytes)
                .max(self.small.span_bytes),
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
        alignment: usize,
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

        let first_offset =
            self.reserve_space_range_aligned(pages.len() * self.allocator.page_bytes(), alignment)?;

        // materialize the full large allocation before publishing it
        self.mapping
            .materialize(first_offset, pages.len() * self.allocator.page_bytes())?;

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
        alignment: usize,
        payload: Payload<'_>,
        reference_map: &ReferenceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(write_offset) =
            self.reserve_young_range(byte_len, alignment, reference_map, tracks_shared_edges)?
        else {
            return Ok(None);
        };

        // initialize only the touched pages
        match payload {
            Payload::Bytes(bytes) => unsafe {
                self.mapping.write_mapped_bytes(write_offset, bytes);
            },
            Payload::Zeroed => {}
        }

        Ok(Some(HeapReference::new(write_offset)))
    }

    /// Allocate caller bytes at the young-space bump cursor when the request fits.
    #[inline(always)]
    fn allocate_young_range_bytes(
        &mut self,
        byte_len: usize,
        alignment: usize,
        bytes: &[u8],
        reference_map: &ReferenceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(write_offset) =
            self.reserve_young_range(byte_len, alignment, reference_map, tracks_shared_edges)?
        else {
            return Ok(None);
        };

        // initialize only the touched pages
        unsafe {
            self.mapping.write_mapped_bytes(write_offset, bytes);
        }

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
            .min(self.small.span_bytes * 8)
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

    /// Allocate one zeroed payload at the young-space bump cursor.
    #[inline(always)]
    fn allocate_young_zeroed(
        &mut self,
        byte_len: usize,
        alignment: usize,
        reference_map: &ReferenceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<HeapReference>> {
        let Some(write_offset) =
            self.reserve_young_range(byte_len, alignment, reference_map, tracks_shared_edges)?
        else {
            return Ok(None);
        };

        Ok(Some(HeapReference::new(write_offset)))
    }

    /// Reserve one aligned young-space byte range.
    #[inline(always)]
    fn reserve_young_range(
        &mut self,
        byte_len: usize,
        alignment: usize,
        reference_map: &ReferenceMap,
        tracks_shared_edges: bool,
    ) -> HeapResult<Option<usize>> {
        if alignment > self.young.allocation_alignment_bytes {
            return Ok(None);
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        if reference_map.has_tagged_reference() {
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
        let start_index = self.young.start_index(write_offset);

        // materialize the allocation range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install the metadata before exposing the address
        debug_assert!(u32::try_from(byte_len).is_ok());
        self.young.starts.set_in_bounds(start_index);
        self.young.byte_lens[start_index] = byte_len as u32;
        self.young.live.set_in_bounds(start_index);
        self.young.marked.clear_in_bounds(start_index);
        if reference_map.has_local_reference() || tracks_shared_edges {
            write_allocation_reference_bits(
                reference_map,
                &mut self.young.local_reference_bits,
                &mut self.young.shared_reference_bits,
                write_offset,
                byte_len,
            );
        }

        // advance the young-space tail after installing the allocation
        self.young.next_offset = end_offset;

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
        let start_index = self.young.start_index(write_offset);

        // materialize the allocation range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install exact no-scan metadata
        debug_assert!(u32::try_from(byte_len).is_ok());
        self.young.starts.set_in_bounds(start_index);
        self.young.byte_lens[start_index] = byte_len as u32;
        self.young.live.set_in_bounds(start_index);

        self.young.next_offset = end_offset;

        Ok(Some((write_offset, start_index)))
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

    /// Reserve one small-span payload location for the given runtime facts.
    fn reserve_small_payload(
        &mut self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> HeapResult<Option<(SmallSpanClass, usize, usize)>> {
        // resolve the matching size class first
        let Some(class) = self.small_span_class(
            byte_len,
            self.young.allocation_alignment_bytes,
            reference_map,
        ) else {
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
                    let pages = self.allocate_page_run(class.span_bytes)?;

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

                    // materialize the full span before handing out slots
                    self.mapping.materialize(first_offset, class.span_bytes)?;

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (class.span_bytes / class.size_class).max(1);
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let dirty_card_bytes = slot_count * class.size_class;
        let pages = self.allocate_page_run(class.span_bytes)?;
        let first_offset = self.reserve_space_range(class.span_bytes)?;

        // materialize the full span before handing out slots
        self.mapping.materialize(first_offset, class.span_bytes)?;

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
            Payload::Bytes(bytes) if bytes.len() < class.size_class => unsafe {
                write_bytes(
                    (self.mapping.base_address() + mapping_offset) as *mut u8,
                    0,
                    class.size_class,
                );
                self.mapping.write_mapped_bytes(mapping_offset, bytes);
            },
            Payload::Bytes(bytes) => unsafe {
                self.mapping.write_mapped_bytes(mapping_offset, bytes);
            },
            Payload::Zeroed => unsafe {
                write_bytes(
                    (self.mapping.base_address() + mapping_offset) as *mut u8,
                    0,
                    class.size_class,
                );
            },
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
        );

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

/// One reserved fixed-size young slot.
#[derive(Debug, Clone, Copy)]
struct YoungRunSlot {
    /// The allocated heap reference.
    reference: HeapReference,
}

/// Return the offset rounded up to one allocation boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
