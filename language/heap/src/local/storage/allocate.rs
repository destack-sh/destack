use tspp_mir::TraceMap;

use super::{HeapPlace, HeapStorage, LargeBlock, LargeBlockId, PageOwner, SmallSpan};
use crate::{
    Allocation, Bitmap, DropPlan, HeapAllocationError, HeapError, HeapReference,
    HeapRepresentationError, HeapResult, Payload, Slot, SmallAllocationClass, SmallSpanClass,
    clear_slot_reference_bits, write_slot_reference_bits,
};
use tspp_memory::MemoryRange;

impl HeapStorage {
    /// Return the projected retained-byte delta for one block plan.
    pub(crate) fn retained_byte_delta(&self, layout: &Allocation<'_>) -> HeapResult<i64> {
        // reject empty heap blocks
        if layout.is_empty() {
            return Err(HeapError::invalid_allocation(HeapAllocationError::ZeroSize));
        }

        // use one span slot when one is free
        if let Some(small) = layout.class.as_small() {
            if self.has_free_slot(&small) {
                Ok(0)
            } else {
                Ok(small.class.span_size_bytes() as i64)
            }
        }
        // otherwise allocate one dedicated large block
        else {
            Ok(self.round_up_large_block_bytes(layout.byte_len) as i64)
        }
    }

    /// Reserve one slot from the span one small class allocates from.
    #[inline(always)]
    pub(crate) fn reserve_slot(&mut self, small: SmallAllocationClass) -> Option<Slot> {
        // resolve the span this class reserves from
        let span_index = (*self.small.cursors.get(small.cache_index())?)?;
        let span = self.small.spans.get_mut(span_index)?;
        debug_assert_eq!(span.class, small.class);
        if !span.has_free_slot() {
            return None;
        }

        // occupy the slot and advance past it
        let slot_index = span.free_cursor;
        span.occupied.set(slot_index);
        span.marked.clear(slot_index);
        span.occupied_count += 1;
        span.free_cursor = span
            .occupied
            .first_clear_from(slot_index + 1)
            .unwrap_or(span.slot_count);
        self.usage.allocate(small.class.size_class());

        Some(Slot::from_raw(span_index as u32, slot_index as u32))
    }

    /// Return the reference of one span slot.
    #[inline(always)]
    pub(crate) fn slot_reference(&self, slot: Slot) -> HeapReference {
        let span = &self.small.spans[slot.span_index()];

        HeapReference::new(span.slot_offset(slot.slot_index()))
    }

    /// Allocate one heap block.
    pub(crate) fn allocate(
        &mut self,
        layout: &Allocation<'_>,
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

        // place the payload in span or large storage
        let (reference, place) = self.allocate_place(layout, payload)?;

        // track a reference whose layout may hold shared edges, queueing edges written already
        if layout.has_shared_reference {
            self.collector.track_shared_edge_root(reference);
            if payload.byte_len().is_some() {
                self.queue_shared_edge_root(reference);
            }
        }

        // keep the allocations the payload references for the collector
        if let Payload::Bytes(bytes) = payload {
            self.retain_payload(layout.trace_map, bytes)?;
        }

        // publish the block into any active local cycle
        self.publish_allocation(reference, place, layout.trace_map)?;

        Ok(reference)
    }

    /// Allocate storage for one payload and return its reference and place.
    fn allocate_place(
        &mut self,
        layout: &Allocation<'_>,
        payload: Payload<'_>,
    ) -> HeapResult<(HeapReference, HeapPlace)> {
        // allocate from the class span when the payload still fits
        if let Some(small) = layout.class.as_small() {
            let slot = match self.reserve_slot(small) {
                Some(slot) => slot,
                None => {
                    self.refill_cursor(small)?;
                    self.reserve_slot(small)
                        .ok_or(HeapError::internal("refilled span has no free slot"))?
                }
            };
            self.initialize_slot(slot, layout.trace_map, payload)?;

            Ok((self.slot_reference(slot), HeapPlace::Slot(slot)))
        }
        // otherwise allocate one dedicated large block
        else {
            let pages = self.allocate_page_span(layout.byte_len, layout.alignment)?;
            let block_id = self.insert_large_block(
                layout.byte_len,
                pages,
                layout.trace_map.clone(),
                layout.drop,
            )?;
            let Some(block) = self.large_block(block_id) else {
                return Err(HeapError::internal("missing large block"));
            };
            let first_offset = block.first_offset;

            payload.initialize_mapped(&self.memory, first_offset, layout.byte_len);
            self.usage.allocate(layout.byte_len);

            Ok((
                HeapReference::new(first_offset),
                HeapPlace::LargeBlock(block_id),
            ))
        }
    }

    /// Free one heap block.
    pub(crate) fn free(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        if extent.byte_offset != 0 {
            return Err(HeapError::invalid_heap_reference(reference));
        }

        match extent.place {
            // release one span slot
            HeapPlace::Slot(slot) => {
                self.release_slot(slot)?;
                self.collector.remove_shared_edge_root(reference);
                self.usage.free(extent.byte_len as u64);

                Ok(())
            }

            // release one block in large space and its memory pages
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                let pages = block.pages;

                let index = block_id.index()?;
                let Some(block) = self.large.blocks.get_mut(index) else {
                    return Err(HeapError::internal("missing large block"));
                };

                // retire the live large-block slot before releasing its pages
                *block = None;
                self.large.free_large_block_ids.push(block_id.id());

                self.collector.remove_shared_edge_root(reference);

                self.unmap_page_span(&pages);
                self.release_page_span(pages)?;
                self.usage.free(extent.byte_len as u64);

                Ok(())
            }
        }
    }

    /// Release one heap span slot.
    pub(crate) fn release_slot(&mut self, slot: Slot) -> HeapResult<()> {
        // resolve the owning span
        let span_index = slot.span_index();
        let is_cursor = self.is_cursor_span(span_index);
        let Some(span) = self.span_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        // reject a slot the span records as free
        let slot_index = slot.slot_index();
        let was_full = span.occupied_count == span.slot_count;
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::internal("missing small slot"));
        }

        // vacate the slot and its trace metadata
        span.occupied.clear(slot_index);
        span.marked.clear(slot_index);
        span.retained.clear(slot_index);
        span.empty.clear(slot_index);
        let size_class = span.class.size_class();
        clear_slot_reference_bits(
            &mut span.local_reference_bits,
            &mut span.shared_reference_bits,
            slot_index,
            size_class,
        );
        span.occupied_count -= 1;
        span.free_cursor = span.free_cursor.min(slot_index);

        // requeue a span that regained a free slot, the class cursor reserving from it directly
        if was_full && !is_cursor {
            let class = span.class;
            self.small
                .partial_spans
                .entry(class)
                .or_default()
                .push(span_index);
        }

        Ok(())
    }

    /// Return whether one span is the cursor of its class.
    fn is_cursor_span(&self, span_index: usize) -> bool {
        self.small.cursors.contains(&Some(span_index))
    }

    /// Return whether one size class still has one reusable slot without a new span.
    fn has_free_slot(&self, small: &SmallAllocationClass) -> bool {
        // the class cursor answers first, then its partial spans
        let cursor_has_slot = self
            .small
            .cursors
            .get(small.cache_index())
            .copied()
            .flatten()
            .and_then(|span_index| self.small.spans.get(span_index))
            .is_some_and(SmallSpan::has_free_slot);

        cursor_has_slot
            || self
                .small
                .partial_spans
                .get(&small.class)
                .is_some_and(|spans| !spans.is_empty())
    }

    /// Return the page-rounded retained bytes for one heap large block.
    fn round_up_large_block_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
    }

    /// Insert one large block record.
    pub(crate) fn insert_large_block(
        &mut self,
        byte_len: usize,
        pages: MemoryRange,
        trace_map: TraceMap,
        drop: Option<DropPlan>,
    ) -> HeapResult<LargeBlockId> {
        // reuse one freed large block id when possible
        let reused_block_id = self.large.free_large_block_ids.pop();
        let block_id = match reused_block_id {
            Some(block_id) => block_id,
            None => self.large.next_unused_large_block_id,
        };

        if block_id == 0 {
            if reused_block_id.is_some() {
                self.large.free_large_block_ids.push(block_id);
            }

            self.release_page_span(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id },
            ));
        }

        let block_id = LargeBlockId::new(block_id);
        let index = block_id.index()?;
        if index > self.large.blocks.len() {
            if reused_block_id.is_some() {
                self.large.free_large_block_ids.push(block_id.id());
            }

            self.release_page_span(pages)?;

            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: block_id.id() },
            ));
        }

        let first_offset = pages.offset;

        // materialize the full large block before publishing it
        if let Err(error) = self.memory.materialize(first_offset, pages.byte_len) {
            if reused_block_id.is_some() {
                self.large.free_large_block_ids.push(block_id.id());
            }

            self.release_page_span(pages)?;

            return Err(error.into());
        }

        self.map_page_span(&pages, |logical_page_index| PageOwner::LargeBlock {
            block_id,
            logical_page_index,
        });

        // materialize the block record
        let block = LargeBlock {
            first_offset,
            byte_len,
            pages,
            trace_map,
            drop,
            retained: false,
            empty: false,
            mark_epoch: 0,
        };

        // insert or replace the block record
        if index == self.large.blocks.len() {
            self.large.blocks.push(Some(block));
        } else {
            self.large.blocks[index] = Some(block);
        }
        if reused_block_id.is_none() {
            self.large.next_unused_large_block_id += 1;
        }

        Ok(block_id)
    }

    /// Point one small class at a span with a free slot.
    ///
    /// A partial span is reused before a new span is mapped.
    fn refill_cursor(&mut self, small: SmallAllocationClass) -> HeapResult<()> {
        // read the class and its cursor index
        let class = small.class;
        let cache_index = small.cache_index();

        // reuse one partial span, else map a fresh one
        let span_index = match self.small.partial_spans.get_mut(&class).and_then(Vec::pop) {
            Some(span_index) => span_index,
            None => self.allocate_span(&class)?,
        };

        // point the class cursor at the span
        if self.small.cursors.len() <= cache_index {
            self.small.cursors.resize(cache_index + 1, None);
        }
        self.small.cursors[cache_index] = Some(span_index);

        Ok(())
    }

    /// Map one fresh span for the given size class.
    fn allocate_span(&mut self, class: &SmallSpanClass) -> HeapResult<usize> {
        let slot_count = (class.span_size_bytes() / class.size_class()).max(1);
        let scan_word_count = class.size_class().div_ceil(std::mem::size_of::<usize>());
        let pages = self.allocate_page_span(class.span_size_bytes(), self.page_size_bytes())?;
        let first_offset = pages.offset;

        // materialize the full span before handing out slots
        self.memory
            .materialize(first_offset, class.span_size_bytes())?;

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
            retained: Bitmap::with_capacity(slot_count),
            empty: Bitmap::with_capacity(slot_count),
            mark_epoch: 0,
            pages,
        };
        let span_index = self.small.spans.len();
        self.map_page_span(&pages, |logical_page_index| PageOwner::Span {
            span_index,
            logical_page_index,
        });

        self.small.spans.push(span);

        Ok(span_index)
    }

    /// Initialize one reserved span slot payload and its direct trace metadata.
    fn initialize_slot(
        &mut self,
        slot: Slot,
        trace_map: &TraceMap,
        init: Payload<'_>,
    ) -> HeapResult<()> {
        // write the payload across the whole reserved slot
        let offset = self.slot_reference(slot).offset();
        let span = self
            .small
            .spans
            .get_mut(slot.span_index())
            .ok_or(HeapError::internal("missing span"))?;
        let size_class = span.class.size_class();
        init.initialize_mapped(&self.memory, offset, size_class);

        // publish direct trace metadata when the class has no table id
        if span.class.trace_id().is_none() {
            write_slot_reference_bits(
                trace_map,
                &mut span.local_reference_bits,
                &mut span.shared_reference_bits,
                slot.slot_index(),
                size_class,
            );
        }

        Ok(())
    }
}
