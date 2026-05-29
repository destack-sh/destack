use super::{
    LargeBlock, LargeBlockId, RawPageMapEntry, RawSmallSpanClass, RawSpace, RawStorage, SmallSpan,
};
use crate::allocator::{PageSpan, Slot};
use crate::{
    Bitmap, HeapAllocationError, HeapError, HeapRepresentationError, HeapResult, Payload,
    RawAllocationShape, RawPointer,
};

impl RawSpace {
    /// Return the projected retained-byte delta for one raw block.
    pub(crate) fn alloc_retained_byte_delta(&self, shape: RawAllocationShape) -> i64 {
        if let Some(class) = self.small_span_class(shape) {
            return if self.has_available_small_slot(&class) {
                0
            } else {
                class.span_size_bytes as i64
            };
        }

        self.round_up_large_block_bytes(shape.byte_len) as i64
    }

    /// Return the projected retained-byte delta for one raw replacement.
    pub fn replace_retained_byte_delta(
        &self,
        pointer: RawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        // resolve the current live block first
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };

        // project the retained-byte delta for this replacement
        let previous_retained_bytes = self.extent_retained_bytes(extent.storage, extent.byte_len);
        let next_retained_bytes = self.extent_replace_retained_bytes(extent.storage, next_byte_len);

        Ok(next_retained_bytes as i64 - previous_retained_bytes as i64)
    }

    /// Allocate one raw block.
    pub fn allocate(
        &mut self,
        shape: RawAllocationShape,
        payload: Payload<'_>,
    ) -> HeapResult<RawPointer> {
        if let Some(actual) = payload.byte_len()
            && actual != shape.byte_len
        {
            return Err(HeapError::invalid_allocation(
                HeapAllocationError::ByteLengthMismatch {
                    expected: shape.byte_len,
                    actual,
                },
            ));
        }

        let storage = self.allocate_place(shape, payload)?;
        let pointer = self.base_pointer(storage)?;

        // charge the live raw block counters
        self.usage.allocate(shape.byte_len);

        Ok(pointer)
    }

    /// Free one raw block.
    pub fn free(&mut self, pointer: RawPointer) -> HeapResult<()> {
        // resolve the live block first
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };
        let freed_bytes = extent.byte_len as u64;
        self.usage.check_free(freed_bytes);

        match extent.storage {
            // release one small-span slot
            RawStorage::SmallSlot(slot) => {
                self.release_small_slot(slot)?;

                // update heap usage
                self.usage.free(freed_bytes);

                Ok(())
            }

            // release one block in large space and its allocator pages
            RawStorage::LargeBlock(block_id) => {
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

                // update heap usage
                self.usage.free(freed_bytes);

                // release the old physical pages after the live slot is gone
                self.unmap_page_span(first_offset, &pages);
                self.release_page_span(pages)?;

                Ok(())
            }
        }
    }

    /// Allocate one raw storage for the given payload.
    pub(super) fn allocate_place(
        &mut self,
        shape: RawAllocationShape,
        payload: Payload<'_>,
    ) -> HeapResult<RawStorage> {
        if let Some((class, span_index, slot_index)) = self.reserve_small_slot(shape)? {
            let slot = self.initialize_small_slot(&class, span_index, slot_index, payload)?;

            Ok(RawStorage::SmallSlot(slot))
        } else {
            let pages = self.allocate_page_span(shape.byte_len)?;
            let block_id = self.insert_large_block(shape.byte_len, shape.alignment, pages)?;
            let Some(large_block) = self.large_block(block_id) else {
                return Err(HeapError::internal("missing large block"));
            };
            let first_offset = large_block.first_offset;

            self.initialize_mapped_payload(first_offset, shape.byte_len, payload);

            Ok(RawStorage::LargeBlock(block_id))
        }
    }

    /// Insert one raw large block record.
    pub(super) fn insert_large_block(
        &mut self,
        byte_len: usize,
        alignment: usize,
        pages: PageSpan,
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

        let first_offset = self.reserve_space_range_aligned(
            pages.len() * self.allocator.page_size_bytes(),
            alignment,
        )?;

        // materialize the full large block before publishing it
        self.mapping
            .materialize(first_offset, pages.len() * self.allocator.page_size_bytes())?;

        self.map_page_span(first_offset, &pages, |logical_page_index| {
            RawPageMapEntry::LargeBlock {
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
        };

        // append at the unused tail when possible
        if let Err(error) = self.large.blocks.set_or_push(index, block) {
            if reused_block_id {
                self.large.free_large_block_ids.push(block_id.id());
            }

            self.unmap_page_span(first_offset, &pages);
            self.release_page_span(pages)?;

            return Err(error);
        }

        Ok(block_id)
    }

    /// Allocate one raw small slot from one byte slice.
    pub(super) fn allocate_small_bytes(&mut self, bytes: &[u8]) -> HeapResult<Option<Slot>> {
        let shape = RawAllocationShape::bytes(bytes.len());
        let Some((class, span_index, slot_index)) = self.reserve_small_slot(shape)? else {
            return Ok(None);
        };

        self.initialize_small_slot(&class, span_index, slot_index, Payload::Bytes(bytes))
            .map(Some)
    }

    /// Allocate or reuse one non-full raw span for the given size class.
    fn allocate_small_span(&mut self, class: &RawSmallSpanClass) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self.small.partial_spans.get_mut(class).and_then(Vec::pop) {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let first_offset = span.first_offset;
                    let pages = self.allocate_page_span(class.span_size_bytes)?;

                    // materialize the full span before handing out slots
                    self.mapping
                        .materialize(first_offset, class.span_size_bytes)?;

                    self.map_page_span(first_offset, &pages, |logical_page_index| {
                        RawPageMapEntry::SmallSpan {
                            span_index,
                            logical_page_index,
                        }
                    });

                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.unmap_page_span(first_offset, &pages);
                        self.release_page_span(pages)?;

                        return Err(HeapError::internal("missing span"));
                    };

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (class.span_size_bytes / class.size_class).max(1);
        let pages = self.allocate_page_span(class.span_size_bytes)?;
        let first_offset =
            self.reserve_space_range_aligned(class.span_size_bytes, class.size_class)?;

        // materialize the full span before handing out slots
        self.mapping
            .materialize(first_offset, class.span_size_bytes)?;

        let span = SmallSpan {
            first_offset,
            class: class.clone(),
            slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: Bitmap::with_capacity(slot_count),
            pages,
        };
        let span_index = self.small.spans.len();
        self.map_page_span(first_offset, &pages, |logical_page_index| {
            RawPageMapEntry::SmallSpan {
                span_index,
                logical_page_index,
            }
        });

        self.small.spans.push(span);

        Ok(span_index)
    }

    /// Reserve one raw small-span payload for the given byte length.
    fn reserve_small_slot(
        &mut self,
        shape: RawAllocationShape,
    ) -> HeapResult<Option<(RawSmallSpanClass, usize, usize)>> {
        let Some(class) = self.small_span_class(shape) else {
            return Ok(None);
        };
        let span_index = self.allocate_small_span(&class)?;
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        let slot_index = span.free_cursor;

        Ok(Some((class, span_index, slot_index)))
    }

    /// Initialize one reserved raw small-span payload.
    fn initialize_small_slot(
        &mut self,
        class: &RawSmallSpanClass,
        span_index: usize,
        slot_index: usize,
        block: Payload<'_>,
    ) -> HeapResult<Slot> {
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        let slot_offset = span.class.size_class * slot_index;

        let mapping_offset = span.first_offset + slot_offset;

        self.initialize_mapped_payload(mapping_offset, class.byte_len, block);

        let Some(span) = self.small.spans.get_mut(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        // mark the slot as live inside its span
        span.occupied.set(slot_index);
        span.occupied_count += 1;
        span.free_cursor = span
            .occupied
            .first_clear_from(slot_index)
            .unwrap_or(span.slot_count);

        // requeue the span if it still has capacity
        if span.occupied_count < span.slot_count {
            self.small
                .partial_spans
                .entry(class.clone())
                .or_default()
                .push(span_index);
        }

        Slot::new(span_index, slot_index)
    }

    /// Release one raw small slot without releasing its stable pointer id.
    pub(super) fn release_small_slot(&mut self, slot: Slot) -> HeapResult<()> {
        let pages = {
            // resolve the live span first
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::internal("missing span"));
            };
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;
            let class = span.class.clone();

            // ignore stale releases for already-free slots
            if !span.occupied.contains(slot_index) {
                return Err(HeapError::internal("missing small slot"));
            }
            if span.occupied_count == 0 {
                return Err(HeapError::internal("missing small slot"));
            }

            // mark the slot as free inside the span
            span.occupied.clear(slot_index);
            span.occupied_count -= 1;
            span.free_cursor = span.free_cursor.min(slot_index);

            // release fully empty span pages back into the local cache
            if span.occupied_count == 0 {
                span.free_cursor = 0;

                let first_offset = span.first_offset;
                let pages = span.pages;
                span.pages = PageSpan::empty();

                Some((first_offset, pages))
            }
            // otherwise requeue the span once it transitions away from full
            else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;
                if should_requeue {
                    self.small
                        .partial_spans
                        .entry(class)
                        .or_default()
                        .push(slot.span_index());
                }

                None
            }
        };

        if let Some((first_offset, pages)) = pages {
            self.unmap_page_span(first_offset, &pages);
            self.release_page_span(pages)?;
        }

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class: &RawSmallSpanClass) -> bool {
        self.small
            .partial_spans
            .get(class)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Return one homogeneous raw span class for the given shape when it fits.
    fn small_span_class(&self, shape: RawAllocationShape) -> Option<RawSmallSpanClass> {
        let class_index = self
            .small
            .size_classes
            .class_index_for_layout(shape.byte_len, shape.alignment)?;
        let size_class = self.small.size_classes.classes[class_index];

        Some(RawSmallSpanClass {
            size_class: size_class.bytes,
            span_size_bytes: size_class
                .span_size_bytes(self.allocator.page_size_bytes(), self.small.span_size_bytes)
                .max(self.small.span_size_bytes),
            byte_len: shape.byte_len,
        })
    }

    /// Return the retained bytes currently charged to one raw extent.
    fn extent_retained_bytes(&self, storage: RawStorage, byte_len: usize) -> u64 {
        match storage {
            RawStorage::SmallSlot(_) => 0,
            RawStorage::LargeBlock(_) => self.round_up_large_block_bytes(byte_len),
        }
    }

    /// Return the retained bytes for one replacement target extent.
    fn extent_replace_retained_bytes(&self, storage: RawStorage, next_byte_len: usize) -> u64 {
        // small blocks stay in storage when the next payload still fits
        if let RawStorage::SmallSlot(slot) = storage
            && let Some(span) = self.span(slot.span_index())
            && next_byte_len == span.class.byte_len
        {
            return 0;
        }

        // otherwise use the same retained-byte delta model as fresh block
        self.alloc_retained_byte_delta(RawAllocationShape::bytes(next_byte_len)) as u64
    }

    /// Return the page-rounded retained bytes for one raw large block.
    fn round_up_large_block_bytes(&self, byte_len: usize) -> u64 {
        let page_size_bytes = self.allocator.page_size_bytes() as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_size_bytes) * page_size_bytes
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
            Payload::Bytes(bytes) => self.write_mapped_bytes(offset, bytes),
            Payload::Zeroed => self.clear_mapped_bytes(offset, clear_byte_len),
            Payload::Uninit => {}
        }
    }

    /// Write bytes into one mapped payload range.
    #[inline(always)]
    pub(super) fn write_mapped_bytes(&self, offset: usize, bytes: &[u8]) {
        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            self.mapping.write_mapped_bytes(offset, bytes);
        }
    }

    /// Clear one mapped payload range.
    #[inline(always)]
    pub(super) fn clear_mapped_bytes(&self, offset: usize, byte_len: usize) {
        let address = self.mapping.base_address() + offset;

        // SAFETY: block paths materialize the destination before publishing it
        unsafe {
            std::ptr::write_bytes(address as *mut u8, 0, byte_len);
        }
    }
}
