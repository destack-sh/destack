use destack_mir::TraceMap;

use super::{
    CardSet, HeapPageMapEntry, HeapPlace, HeapStorage, LargeBlock, LargeBlockId, Phase, SmallSpan,
    YoungSpan, YoungSpanBits,
};
use crate::allocator::{PageSpan, Slot};
use crate::{
    AllocationPlan, Bitmap, HeapAllocationError, HeapError, HeapReference, HeapRepresentationError,
    HeapResult, Payload, SmallAllocationPlan, SmallSpanClass, align_up,
    clear_allocation_reference_bits, clear_slot_reference_bits, write_allocation_reference_bits,
    write_slot_reference_bits,
};

impl HeapStorage {
    /// Return the projected retained-byte delta for one block plan.
    pub(crate) fn retained_byte_delta(&self, layout: &AllocationPlan<'_>) -> HeapResult<i64> {
        // reject empty heap blocks
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // stay in young space when the payload still fits
        if self.young_fits(layout.byte_len, layout.alignment) {
            Ok(0)
        }
        // use one traced small span when the payload still fits
        else if let Some(small) = layout.class.small() {
            if self.has_available_small_slot(&small) {
                Ok(0)
            } else {
                Ok(small.class.span_size_bytes as i64)
            }
        }
        // otherwise allocate one dedicated large block
        else {
            Ok(self.round_up_large_block_bytes(layout.byte_len) as i64)
        }
    }

    /// Reserve one reference from the active young cursor.
    #[inline(always)]
    pub(crate) fn reserve_young_cursor(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        let reference = self
            .young
            .cursor
            .as_mut()?
            .reserve_matching_reference(byte_len, class)?;

        Some(reference)
    }

    /// Reserve one noscan reference from the active young cursor.
    #[inline(always)]
    pub(crate) fn reserve_young_noscan_cursor(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
    ) -> Option<HeapReference> {
        let reference = self
            .young
            .cursor
            .as_mut()?
            .reserve_noscan_reference(byte_len, class)?;

        Some(reference)
    }

    /// Allocate one fixed-size payload from young space.
    #[inline(always)]
    fn reserve_young_span(&mut self, layout: &AllocationPlan<'_>) -> HeapResult<Option<YoungSlot>> {
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

        // stay on the active span cursor without consulting metadata
        if let Some(cursor) = &mut self.young.cursor
            && cursor.matches(class, byte_len)
            && let Some(reference) = cursor.reserve_reference()
        {
            let span_index = cursor.span_index;
            let Some(span) = self.young.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_index = (reference.offset() - span.first_offset) / class.size_class;

            return Ok(Some(YoungSlot {
                reference,
                span_index,
                slot_index,
            }));
        }

        if byte_len > self.max_young_allocation_bytes {
            return Ok(None);
        }

        // reuse the cached class span when it still has space
        if let Some(span_index) = self
            .young
            .span_cache
            .get(cache_index)
            .and_then(|span_index| *span_index)
            && let Some(slot) = self.reserve_young_slot(byte_len, class, span_index)
        {
            return Ok(Some(slot));
        }

        let Some(span_index) = self.allocate_young_span(byte_len, class, cache_index)? else {
            return Ok(None);
        };

        Ok(self.reserve_young_slot(byte_len, class, span_index))
    }

    /// Allocate one fixed-size young span for one small class.
    fn allocate_young_span(
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
        let span_bytes = configured_bytes.min(available_bytes);
        let span_bytes = span_bytes / self.young.page_size_bytes * self.young.page_size_bytes;
        if span_bytes < size_class {
            return Ok(None);
        }

        let slot_count = span_bytes / size_class;
        let page_start = first_offset / self.young.page_size_bytes;
        let page_count = span_bytes / self.young.page_size_bytes;

        // materialize the whole span before publishing its slots
        self.materialize_young_range(first_offset, span_bytes)?;
        // SAFETY: the span range was materialized above
        unsafe {
            self.mapping.zero_mapped_bytes(first_offset, span_bytes);
        }

        // publish the span before handing out its first reference
        let span_index = self.young.spans.len();
        self.young.spans.push(YoungSpan {
            first_offset,
            byte_len,
            next_offset: first_offset,
            end_offset: first_offset + span_bytes,
            class,
        });
        self.young.span_bits.push(YoungSpanBits {
            freed: Bitmap::with_capacity(slot_count),
            marked: Bitmap::with_capacity(slot_count),
        });

        for page_index in page_start..page_start + page_count {
            self.young.page_spans[page_index] = Some(span_index);
        }

        if self.young.span_cache.len() <= cache_index {
            self.young.span_cache.resize(cache_index + 1, None);
        }
        self.young.span_cache[cache_index] = Some(span_index);
        self.young.next_offset = first_offset + span_bytes;

        Ok(Some(span_index))
    }

    /// Reserve one fixed-size young span slot.
    #[inline(always)]
    fn reserve_young_slot(
        &mut self,
        byte_len: usize,
        class: SmallSpanClass,
        span_index: usize,
    ) -> Option<YoungSlot> {
        self.flush_young_cursor();
        self.young.activate_cursor(byte_len, class, span_index)?;
        let reference = self.young.cursor.as_mut()?.reserve_reference()?;
        let span = self.young.span(span_index)?;
        let slot_index = (reference.offset() - span.first_offset) / class.size_class;

        Some(YoungSlot {
            reference,
            span_index,
            slot_index,
        })
    }

    /// Publish one traced young span block to active collectors.
    #[inline(always)]
    fn publish_young_cache_allocation(
        &mut self,
        slot: YoungSlot,
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
            HeapPlace::YoungSlot(Slot::new(slot.span_index, slot.slot_index)?),
            trace_map,
        )
    }

    /// Allocate one heap block.
    pub(crate) fn allocate(
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

        self.allocate_mature(layout, payload, has_initialized_bytes)
    }

    /// Reserve one heap block in young space.
    pub(crate) fn reserve_young_payload(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<Option<HeapReference>> {
        let trace_map = layout.trace_map;
        let tracks_shared_edges = layout.has_shared_reference;
        let has_initialized_bytes = payload.byte_len().is_some();

        // reserve from a fixed-size young span
        if let Some(slot) = self.reserve_young_span(layout)? {
            payload.initialize_zeroed_mapped(&self.mapping, slot.reference.offset());

            if !layout.is_noscan {
                self.publish_young_cache_allocation(slot, trace_map, has_initialized_bytes)?;
            } else if self.collector.major_phase != Phase::Idle {
                let Some(bits) = self.young.span_bits_mut(slot.span_index) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.marked.set_in_bounds(slot.slot_index);
            }

            return Ok(Some(slot.reference));
        }

        // reserve a no-scan young range when fixed-size spans do not fit
        if layout.is_noscan
            && let Some(reservation) =
                self.reserve_young_noscan_range(layout.byte_len, layout.alignment)?
        {
            let reference = HeapReference::new(reservation.first_offset);

            payload.initialize_mapped(&self.mapping, reservation.first_offset, layout.byte_len);

            if self.collector.major_phase != Phase::Idle {
                self.young.marked.set_in_bounds(reservation.range_index);
            }
            self.record_young_range_allocation(layout.byte_len);

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
                HeapPlace::YoungRange {
                    first_offset: reference.offset(),
                },
                trace_map,
            )?;

            return Ok(Some(reference));
        }

        Ok(None)
    }

    /// Allocate one mature heap block from one block plan.
    pub(crate) fn allocate_mature(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
        has_initialized_bytes: bool,
    ) -> HeapResult<HeapReference> {
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        let allocation = self.allocate_mature_place(layout, payload)?;
        let reference = self.base_reference(allocation.place)?;

        // track every live reference whose layout may contain shared edges
        if layout.has_shared_reference {
            self.collector.track_shared_edge_root(reference);
        }

        // queue newly published shared edges during an active shared cycle
        if layout.has_shared_reference && has_initialized_bytes {
            self.queue_shared_edge_root(reference);
        }

        self.publish_major_allocation(reference, allocation.place, layout.trace_map)?;
        self.record_mature_allocation(allocation.charged_bytes);

        Ok(reference)
    }

    /// Free one heap block.
    pub(crate) fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        self.flush_young_cursor();

        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        match extent.storage {
            // retire one young range until the next young sweep
            HeapPlace::YoungRange { first_offset } => {
                let Some(range) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                self.young.live.clear(range.index);
                self.young.marked.clear(range.index);
                clear_allocation_reference_bits(
                    &mut self.young.local_reference_bits,
                    &mut self.young.shared_reference_bits,
                    range.range.first_offset,
                    range.range.byte_len,
                );

                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(extent.byte_len);

                Ok(())
            }

            // retire one young fixed-size slot until the next young sweep
            HeapPlace::YoungSlot(slot) => {
                let Some(bits) = self.young.span_bits_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                bits.freed.set(slot.slot_index());
                bits.marked.clear(slot.slot_index());
                self.collector.remove_shared_edge_root(reference);
                self.record_young_free(extent.byte_len);

                Ok(())
            }

            // release one small-span slot
            HeapPlace::MatureSlot(slot) => {
                self.release_small_slot(slot)?;
                self.collector.remove_shared_edge_root(reference);
                self.record_mature_free(extent.byte_len);

                Ok(())
            }

            // release one block in large space and its allocator pages
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                let first_offset = block.first_offset;
                let pages = block.pages;

                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                // retire the live large-block slot before releasing its pages
                block.retire();
                self.large.free_large_block_ids.push(block_id.id());

                self.collector.remove_shared_edge_root(reference);

                self.unmap_page_span(first_offset, &pages);
                self.release_page_span(pages)?;
                self.record_mature_free(extent.byte_len);

                Ok(())
            }
        }
    }

    /// Allocate one mature heap storage for the given payload.
    fn allocate_mature_place(
        &mut self,
        layout: &AllocationPlan<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<MatureAllocation> {
        // reject inconsistent block
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

            Ok(MatureAllocation {
                place: HeapPlace::MatureSlot(slot),
                charged_bytes: class.size_class,
            })
        }
        // otherwise allocate one dedicated large block
        else {
            let pages = self.allocate_page_span(layout.byte_len)?;
            let block_id = self.insert_large_block(
                layout.byte_len,
                layout.alignment,
                pages,
                layout.trace_map.clone(),
                true,
            )?;
            let Some(block) = self.large_block(block_id) else {
                return Err(HeapError::internal("missing large block"));
            };
            let first_offset = block.first_offset;

            payload.initialize_mapped(&self.mapping, first_offset, layout.byte_len);

            Ok(MatureAllocation {
                place: HeapPlace::LargeBlock(block_id),
                charged_bytes: layout.byte_len,
            })
        }
    }

    /// Release one heap small slot.
    pub(crate) fn release_small_slot(&mut self, slot: Slot) -> HeapResult<()> {
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
                span.pages = PageSpan::empty();

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
            self.unmap_page_span(first_offset, &pages);
            self.release_page_span(pages)?;
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

    /// Report whether one byte length still fits the young space tail.
    #[inline(always)]
    fn young_fits(&self, byte_len: usize, alignment: usize) -> bool {
        if self.collector.minor_phase != Phase::Idle {
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

    /// Return the page-rounded retained bytes for one heap large block.
    fn round_up_large_block_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.allocator.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }

    /// Insert one large block record.
    pub(crate) fn insert_large_block(
        &mut self,
        byte_len: usize,
        alignment: usize,
        pages: PageSpan,
        trace_map: TraceMap,
        remember: bool,
    ) -> HeapResult<LargeBlockId> {
        // reuse one freed large block id when possible
        let (block_id, reused_block_id) =
            if let Some(block_id) = self.large.free_large_block_ids.pop() {
                (block_id, true)
            }
            // otherwise allocate from the unused tail
            else {
                let block_id = self.large.next_unused_large_block_id;
                let next_block_id = self.large.next_unused_large_block_id + 1;

                self.large.next_unused_large_block_id = next_block_id;
                (block_id, false)
            };

        if block_id == 0 {
            if reused_block_id {
                self.large.free_large_block_ids.push(block_id);
            }

            self.release_page_span(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id },
            ));
        }

        let block_id = LargeBlockId::new(block_id);
        let index = block_id.index()?;

        let first_offset = self.reserve_address_range_aligned(
            pages.len() * self.allocator.page_size_bytes(),
            alignment,
        )?;

        // materialize the full large block before publishing it
        self.mapping
            .materialize(first_offset, pages.len() * self.allocator.page_size_bytes())?;

        self.map_page_span(first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::LargeBlock {
                block_id,
                logical_page_index,
            }
        });

        // materialize the block record
        let block = LargeBlock {
            is_live: true,
            first_offset,
            byte_len,
            pages,
            trace_map,
            mark_epoch: 0,
            dirty_cards: CardSet::with_len(byte_len),
            is_dirty_queued: false,
        };

        if let Err(error) = self.large.blocks.set_or_push(index, block) {
            if reused_block_id {
                self.large.free_large_block_ids.push(block_id.id());
            }

            self.unmap_page_span(first_offset, &pages);
            self.release_page_span(pages)?;

            return Err(error);
        }

        // remember new mature blocks conservatively
        if remember {
            self.mark_large_block_dirty(block_id, 0, byte_len)?;
        }

        Ok(block_id)
    }

    /// Allocate one mature payload copied out of young space.
    pub(crate) fn allocate_promoted_payload(
        &mut self,
        byte_len: usize,
        trace_map: &TraceMap,
        bytes: &[u8],
    ) -> HeapResult<HeapPlace> {
        let storage = if !trace_map.has_reference()
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

            HeapPlace::MatureSlot(slot)
        } else {
            let pages = self.allocate_page_span(byte_len)?;
            let block_id = self.insert_large_block(
                byte_len,
                self.allocator.page_size_bytes(),
                pages,
                trace_map.clone(),
                false,
            )?;
            let Some(block) = self.large_block(block_id) else {
                return Err(HeapError::internal("missing large block"));
            };

            Payload::Bytes(bytes).initialize_mapped(&self.mapping, block.first_offset, byte_len);
            self.record_mature_allocation(byte_len);

            HeapPlace::LargeBlock(block_id)
        };

        Ok(storage)
    }

    /// Allocate at the young space bump cursor when the request fits.
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

        payload.initialize_mapped(&self.mapping, write_offset, byte_len);

        Ok(Some(HeapReference::new(write_offset)))
    }

    /// Materialize the young space pages needed by one block.
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
        let frame_size_bytes = self.mapping.frame_size_bytes();
        let frame_start = offset / frame_size_bytes * frame_size_bytes;
        let frame_end = materialize_end.div_ceil(frame_size_bytes) * frame_size_bytes;

        // publish the new materialized prefix
        self.mapping
            .materialize(frame_start, frame_end - frame_start)?;
        self.young.mapped_until = frame_end;

        Ok(())
    }

    /// Reserve one aligned young space byte range.
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

        // reject blocks that do not fit the young space tail
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
        // materialize the block range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install the metadata before exposing the address
        let _range_index = self.young.push_range(write_offset, byte_len);
        if trace_map.has_local_reference() || tracks_shared_edges {
            write_allocation_reference_bits(
                trace_map,
                &mut self.young.local_reference_bits,
                &mut self.young.shared_reference_bits,
                write_offset,
            );
        }

        // advance the young space tail after installing the block
        self.young.next_offset = end_offset;
        self.record_young_range_allocation(byte_len);

        Ok(Some(write_offset))
    }

    /// Reserve one aligned no-scan young space byte range.
    #[inline(always)]
    fn reserve_young_noscan_range(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<Option<YoungRangeReservation>> {
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
        // materialize the block range before publishing metadata
        self.materialize_young_range(write_offset, byte_len)?;

        // install exact no-scan metadata
        let range_index = self.young.push_range(write_offset, byte_len);

        self.young.next_offset = end_offset;

        Ok(Some(YoungRangeReservation {
            first_offset: write_offset,
            range_index,
        }))
    }

    /// Allocate or reuse one non-full heap span for the given size class.
    fn allocate_small_span(&mut self, class: &SmallSpanClass) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self.small.partial_spans.get_mut(class).and_then(Vec::pop) {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let first_offset = span.first_offset;
                    let pages = self.allocate_page_span(class.span_size_bytes)?;

                    self.map_page_span(first_offset, &pages, |logical_page_index| {
                        HeapPageMapEntry::MatureSpan {
                            span_index,
                            logical_page_index,
                        }
                    });

                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.unmap_page_span(first_offset, &pages);
                        self.release_page_span(pages)?;

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
        let pages = self.allocate_page_span(class.span_size_bytes)?;
        let first_offset = self.reserve_address_range(class.span_size_bytes)?;

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
        self.map_page_span(first_offset, &pages, |logical_page_index| {
            HeapPageMapEntry::MatureSpan {
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
    ) -> HeapResult<Slot> {
        let span = self
            .small
            .spans
            .get(span_index)
            .ok_or(HeapError::internal("missing span"))?;
        let slot_offset = span.class.size_class * slot_index;
        let mapping_offset = span.first_offset + slot_offset;

        init.initialize_mapped(&self.mapping, mapping_offset, class.size_class);

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

        let slot = Slot::new(span_index, slot_index)?;

        // remember new mature blocks conservatively
        if remember {
            self.mark_span_slot_dirty(span_index, slot_index, 0, byte_len, trace_map)?;
        }

        Ok(slot)
    }
}

/// One mature heap allocation result.
#[derive(Debug, Clone, Copy)]
struct MatureAllocation {
    /// The physical heap storage.
    place: HeapPlace,
    /// The byte count charged to mature allocation accounting.
    charged_bytes: usize,
}

/// One reserved variable-size young range.
#[derive(Debug, Clone, Copy)]
struct YoungRangeReservation {
    /// The first byte offset inside heap storage.
    first_offset: usize,
    /// The range index inside young space.
    range_index: usize,
}

/// One reserved fixed-size young slot.
#[derive(Debug, Clone, Copy)]
struct YoungSlot {
    /// The allocated heap reference.
    reference: HeapReference,
    /// The owning young span index.
    span_index: usize,
    /// The slot index inside the owning young span.
    slot_index: usize,
}
