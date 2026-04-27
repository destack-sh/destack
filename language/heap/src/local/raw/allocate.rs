use super::{
    LargeAllocation, LargeAllocationId, RawPageMapEntry, RawPlace, RawSmallSpanClass, RawSpace,
    SmallSpan,
};
use crate::allocator::{PageRun, SpanSlot};
use crate::{AccountingRegion, Bitmap, HeapError, HeapResult, Payload, RawPointer};

impl RawSpace {
    /// Return the projected retained-byte delta for one raw allocation.
    pub(crate) fn alloc_retained_byte_delta(&self, byte_len: usize) -> i64 {
        if let Some(class) = self.small_span_class(byte_len) {
            return if self.has_available_small_slot(byte_len) {
                0
            } else {
                class.span_bytes as i64
            };
        }

        self.round_up_large_allocation_bytes(byte_len) as i64
    }

    /// Return the projected retained-byte delta for one raw replacement.
    pub fn replace_retained_byte_delta(
        &self,
        pointer: RawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        // resolve the current live allocation first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        // project the retained-byte delta for this replacement
        let previous_retained_bytes =
            self.location_retained_bytes(location.place, location.byte_len);
        let next_retained_bytes =
            self.location_replace_retained_bytes(location.place, next_byte_len);

        Ok(next_retained_bytes as i64 - previous_retained_bytes as i64)
    }

    /// Allocate one raw allocation.
    pub fn allocate(&mut self, byte_len: usize, payload: Payload<'_>) -> HeapResult<RawPointer> {
        let place = self.allocate_place(byte_len, payload)?;
        let pointer = self.base_pointer(place)?;

        // charge the live raw allocation counters
        self.usage.allocate(byte_len, AccountingRegion::Raw);

        Ok(pointer)
    }

    /// Free one raw allocation.
    pub fn free(&mut self, pointer: RawPointer) -> HeapResult<bool> {
        // resolve the live allocation first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let freed_bytes = location.byte_len as u64;
        self.usage.check_free(freed_bytes, AccountingRegion::Raw);

        match location.place {
            // release one small-span slot
            RawPlace::Small(slot) => {
                self.release_small_slot(slot)?;

                // update heap usage
                self.usage.free(freed_bytes, AccountingRegion::Raw);

                Ok(true)
            }

            // release one allocation in large space and its allocator pages
            RawPlace::Large(allocation_id) => {
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

                // update heap usage
                self.usage.free(freed_bytes, AccountingRegion::Raw);

                // release the old physical pages after the live slot is gone
                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                Ok(true)
            }
        }
    }

    /// Allocate one raw place for the given payload.
    pub(super) fn allocate_place(
        &mut self,
        byte_len: usize,
        payload: Payload<'_>,
    ) -> HeapResult<RawPlace> {
        if let Some((class, span_index, slot_index)) = self.reserve_small_slot(byte_len)? {
            let slot = self.initialize_small_slot(&class, span_index, slot_index, payload)?;

            Ok(RawPlace::Small(slot))
        } else {
            let pages = self.allocate_large_pages(byte_len)?;
            let allocation_id = self.insert_large_allocation(byte_len, pages)?;
            let Some(large_allocation) = self.large_allocation(allocation_id) else {
                return Err(HeapError::MissingLargeAllocation {
                    allocation_id: allocation_id.id(),
                });
            };
            let first_offset = large_allocation.first_offset;

            // initialize bytes before returning the raw pointer
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

            Ok(RawPlace::Large(allocation_id))
        }
    }

    /// Insert one raw large allocation record.
    pub(super) fn insert_large_allocation(
        &mut self,
        len: usize,
        pages: PageRun,
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
            RawPageMapEntry::Large {
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
        };

        // append at the unused tail when possible
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

        Ok(allocation_id)
    }

    /// Allocate one raw small slot from one byte slice.
    pub(super) fn allocate_small_bytes(&mut self, bytes: &[u8]) -> HeapResult<Option<SpanSlot>> {
        let Some((class, span_index, slot_index)) = self.reserve_small_slot(bytes.len())? else {
            return Ok(None);
        };

        self.initialize_small_slot(&class, span_index, slot_index, Payload::Bytes(bytes))
            .map(Some)
    }

    /// Allocate or reuse one non-full raw span for the given size class.
    fn allocate_small_span(&mut self, class: &RawSmallSpanClass) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self
            .small
            .partial_spans
            .get_mut(&class.byte_len)
            .and_then(Vec::pop)
        {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let first_offset = span.first_offset;
                    let pages = self.allocate_page_run_zeroed(class.span_bytes)?;
                    self.map_page_run(first_offset, &pages, |logical_page_index| {
                        RawPageMapEntry::Small {
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
        let pages = self.allocate_page_run_zeroed(class.span_bytes)?;
        let first_offset = self.reserve_space_range(class.span_bytes)?;
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
        self.map_page_run(first_offset, &pages, |logical_page_index| {
            RawPageMapEntry::Small {
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
        byte_len: usize,
    ) -> HeapResult<Option<(RawSmallSpanClass, usize, usize)>> {
        let Some(class) = self.small_span_class(byte_len) else {
            return Ok(None);
        };
        let span_index = self.allocate_small_span(&class)?;
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
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
        allocation: Payload<'_>,
    ) -> HeapResult<SpanSlot> {
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_offset = span.class.size_class * slot_index;

        let mapping_offset = span.first_offset + slot_offset;

        match allocation {
            Payload::Bytes(bytes) => self.mapping.write(mapping_offset, bytes)?,
            Payload::Zeroed => self.mapping.zero(mapping_offset, class.byte_len)?,
        }

        let Some(span) = self.small.spans.get_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
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
                .entry(class.byte_len)
                .or_default()
                .push(span_index);
        }

        SpanSlot::new(span_index, slot_index)
    }

    /// Release one raw small slot without releasing its stable pointer id.
    pub(super) fn release_small_slot(&mut self, slot: SpanSlot) -> HeapResult<()> {
        let pages = {
            // resolve the live span first
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                });
            };
            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;
            let class = span.class.clone();

            // ignore stale releases for already-free slots
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

            // mark the slot as free inside the span
            span.occupied.clear(slot_index);
            span.occupied_count -= 1;
            span.free_cursor = span.free_cursor.min(slot_index);

            // release fully empty span pages back into the local cache
            if span.occupied_count == 0 {
                span.free_cursor = 0;

                let first_offset = span.first_offset;
                let pages = span.pages;
                span.pages = PageRun::empty();

                Some((first_offset, pages))
            }
            // otherwise requeue the span once it transitions away from full
            else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;
                if should_requeue {
                    self.small
                        .partial_spans
                        .entry(class.byte_len)
                        .or_default()
                        .push(slot.span_index());
                }

                None
            }
        };

        if let Some((first_offset, pages)) = pages {
            self.unmap_page_run(first_offset, &pages);
            self.release_page_run(pages)?;
        }

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, byte_len: usize) -> bool {
        self.small
            .partial_spans
            .get(&byte_len)
            .is_some_and(|spans| !spans.is_empty())
    }

    /// Return one homogeneous raw span class for the given byte length when it fits.
    fn small_span_class(&self, byte_len: usize) -> Option<RawSmallSpanClass> {
        let class_index = self.small.size_classes.class_index_for(byte_len)?;
        let size_class = self.small.size_classes.classes[class_index];

        Some(RawSmallSpanClass {
            size_class: size_class.bytes,
            span_bytes: size_class.span_bytes(self.allocator.page_bytes(), self.small.span_bytes),
            byte_len,
        })
    }

    /// Return the retained bytes currently charged to one raw location.
    fn location_retained_bytes(&self, place: RawPlace, byte_len: usize) -> u64 {
        match place {
            RawPlace::Small(_) => 0,
            RawPlace::Large(_) => self.round_up_large_allocation_bytes(byte_len),
        }
    }

    /// Return the retained bytes for one replacement target location.
    fn location_replace_retained_bytes(&self, place: RawPlace, next_byte_len: usize) -> u64 {
        // small allocations stay in place when the next payload still fits
        if let RawPlace::Small(slot) = place
            && let Some(span) = self.span(slot.span_index())
            && next_byte_len == span.class.byte_len
        {
            return 0;
        }

        // otherwise use the same retained-byte delta model as fresh allocation
        self.alloc_retained_byte_delta(next_byte_len) as u64
    }

    /// Return the page-rounded retained bytes for one raw large allocation.
    fn round_up_large_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Allocate one dedicated raw page run.
    fn allocate_large_pages(&mut self, byte_len: usize) -> HeapResult<PageRun> {
        self.allocate_page_run_zeroed(byte_len)
    }
}
