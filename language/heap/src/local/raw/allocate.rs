use super::{
    LargeAllocation, LargeAllocationId, RawPageOwner, RawPlace, RawSmallSpanClass, RawSpace,
    SmallSpan,
};
use crate::allocator::{PageView, SpanSlot};
use crate::{AccountingRegion, Bitmap, HeapError, HeapResult, Payload, RawPointer};

impl RawSpace {
    /// Return the projected mapped-byte delta for one raw allocation.
    pub(crate) fn alloc_mapped_byte_delta(&self, byte_len: usize) -> i64 {
        if self.small_span_class(byte_len).is_some() {
            return if self.has_available_small_slot(byte_len) {
                0
            } else {
                self.small.span_bytes as i64
            };
        }

        self.round_up_large_allocation_bytes(byte_len) as i64
    }

    /// Return the projected mapped-byte delta for one raw replacement.
    pub fn replace_mapped_byte_delta(
        &self,
        pointer: RawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        // resolve the current live allocation first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        // project the mapped-byte delta for this replacement
        let previous_mapped_bytes = self.location_mapped_bytes(location.place, location.byte_len);
        let next_mapped_bytes =
            self.location_replace_mapped_bytes(location.place, location.byte_len, next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Allocate one raw allocation.
    pub fn allocate(&mut self, byte_len: usize, allocation: Payload<'_>) -> HeapResult<RawPointer> {
        if let Some(actual) = allocation.byte_len()
            && actual != byte_len
        {
            return Err(HeapError::InvalidAllocationBytes {
                expected: byte_len,
                actual,
            });
        }

        let place = self.allocate_place(byte_len, allocation)?;
        let pointer = self.base_pointer(place)?;

        // charge the live raw allocation counters
        self.usage.allocate(byte_len, AccountingRegion::Raw)?;

        Ok(pointer)
    }

    /// Free one raw allocation.
    pub fn free(&mut self, pointer: RawPointer) -> HeapResult<bool> {
        // resolve the live allocation first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let freed_bytes = location.byte_len as u64;
        self.usage.check_free(freed_bytes, AccountingRegion::Raw)?;

        match location.place {
            // release one small-span slot
            RawPlace::Small(slot) => {
                self.release_small_slot(slot)?;

                // update heap usage
                self.usage.free(freed_bytes, AccountingRegion::Raw)?;

                Ok(true)
            }

            // release one allocation in large space and its allocator pages
            RawPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                let pages = allocation.pages.clone();

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
                self.usage.free(freed_bytes, AccountingRegion::Raw)?;

                // release the old physical pages after the live slot is gone
                self.unmap_page_view(&pages)?;
                self.release_page_view(pages)?;

                Ok(true)
            }
        }
    }

    /// Allocate one raw place for the given payload.
    fn allocate_place(&mut self, byte_len: usize, allocation: Payload<'_>) -> HeapResult<RawPlace> {
        if let Some((class, span_index, slot_index)) = self.reserve_small_slot(byte_len)? {
            let slot = self.initialize_small_slot(&class, span_index, slot_index, allocation)?;

            return Ok(RawPlace::Small(slot));
        }

        let pages = self.allocate_large_pages(byte_len, allocation)?;
        let allocation_id = self.store_large_allocation(byte_len, pages)?;

        Ok(RawPlace::Large(allocation_id))
    }

    /// Store one raw large allocation in large space and return its allocation id.
    pub(super) fn store_large_allocation(
        &mut self,
        len: usize,
        pages: PageView,
    ) -> HeapResult<LargeAllocationId> {
        // reuse one freed large allocation id when possible
        let (allocation_id, reused_allocation_id) =
            if let Some(allocation_id) = self.large.free_large_allocation_ids.pop() {
                (allocation_id, true)
            }
            // otherwise allocate from the unused tail
            else {
                let allocation_id = self.large.next_unused_large_allocation_id;
                let Some(next_allocation_id) =
                    self.large.next_unused_large_allocation_id.checked_add(1)
                else {
                    self.release_page_view(pages)?;

                    return Err(HeapError::InvalidLargeAllocationId { id: allocation_id });
                };

                self.large.next_unused_large_allocation_id = next_allocation_id;
                (allocation_id, false)
            };

        if allocation_id == 0 {
            if reused_allocation_id {
                self.large.free_large_allocation_ids.push(allocation_id);
            }

            self.release_page_view(pages)?;

            return Err(HeapError::InvalidLargeAllocationId { id: allocation_id });
        }

        let allocation_id = LargeAllocationId::new(allocation_id);
        let index = allocation_id.index()?;

        if let Err(error) = self.map_page_view(&pages, |logical_page_index| RawPageOwner::Large {
            allocation_id,
            logical_page_index,
        }) {
            if reused_allocation_id {
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
            }

            self.release_page_view(pages)?;

            return Err(error);
        }

        // materialize the stored allocation record
        let allocation = LargeAllocation {
            is_live: true,
            len,
            pages: pages.clone(),
        };

        // append at the unused tail when possible
        if let Err(error) = self.large.allocations.set_or_push(index, allocation) {
            if reused_allocation_id {
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
            }

            self.unmap_page_view(&pages)?;
            self.release_page_view(pages)?;

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
                    let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
                    if let Err(error) =
                        self.map_page_view(&pages, |logical_page_index| RawPageOwner::Small {
                            span_index,
                            logical_page_index,
                        })
                    {
                        self.release_page_view(pages)?;

                        return Err(error);
                    }

                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.unmap_page_view(&pages)?;
                        self.release_page_view(pages)?;

                        return Err(HeapError::MissingSpan { span_index });
                    };

                    span.pages = pages.clone();
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (self.small.span_bytes / class.size_class).max(1);
        let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
        let span = SmallSpan {
            class: class.clone(),
            slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: Bitmap::with_capacity(slot_count),
            pages: pages.clone(),
        };
        let span_index = self.small.spans.len();
        if let Err(error) = self.map_page_view(&pages, |logical_page_index| RawPageOwner::Small {
            span_index,
            logical_page_index,
        }) {
            self.release_page_view(pages)?;

            return Err(error);
        }

        if let Err(error) = self.small.spans.push(span) {
            self.unmap_page_view(&pages)?;
            self.release_page_view(pages)?;

            return Err(error);
        }

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
        // resolve the live span first
        let Some(span) = self.small.spans.get_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_offset =
            span.class
                .size_class
                .checked_mul(slot_index)
                .ok_or(HeapError::InvalidSmallSlot {
                    span_index,
                    slot_index,
                })?;

        // initialize the reserved slot payload
        allocation.initialize(&self.allocator, &mut span.pages, slot_offset)?;

        // mark the slot as live inside its span
        span.occupied.set(slot_index);
        span.occupied_count =
            span.occupied_count
                .checked_add(1)
                .ok_or(HeapError::InvariantOverflow {
                    context: "raw span occupancy",
                })?;
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

                let pages = span.pages.clone();
                span.pages = PageView::empty();

                Some(pages)
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

        if let Some(pages) = pages {
            self.unmap_page_view(&pages)?;
            self.release_page_view(pages)?;
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

        Some(RawSmallSpanClass {
            size_class: self.small.size_classes.classes[class_index].bytes,
            byte_len,
        })
    }

    /// Return the mapped bytes currently charged to one raw location.
    fn location_mapped_bytes(&self, place: RawPlace, byte_len: usize) -> u64 {
        match place {
            RawPlace::Small(_) => 0,
            RawPlace::Large(_) => self.round_up_large_allocation_bytes(byte_len),
        }
    }

    /// Return the mapped bytes for one replacement target location.
    fn location_replace_mapped_bytes(
        &self,
        place: RawPlace,
        previous_byte_len: usize,
        next_byte_len: usize,
    ) -> u64 {
        // small allocations stay in place when the next payload still fits
        if let RawPlace::Small(slot) = place
            && let Some(span) = self.span(slot.span_index())
            && next_byte_len == span.class.byte_len
        {
            return 0;
        }

        // otherwise use the same mapped-byte delta model as fresh allocation
        let _ = previous_byte_len;

        self.alloc_mapped_byte_delta(next_byte_len) as u64
    }

    /// Return the page-rounded mapped bytes for one raw large allocation.
    fn round_up_large_allocation_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Allocate one dedicated large-allocation page view for explicit bytes or one zeroed length.
    fn allocate_large_pages(
        &mut self,
        byte_len: usize,
        allocation: Payload<'_>,
    ) -> HeapResult<PageView> {
        match allocation {
            Payload::Bytes(bytes) => self.allocate_page_view_bytes(bytes),
            Payload::Zeroed => self.allocate_page_view_zeroed(byte_len),
            Payload::PageView { .. } => {
                let mut pages = self.allocate_page_view_zeroed(byte_len)?;
                if let Err(error) = allocation.initialize(&self.allocator, &mut pages, 0) {
                    self.release_page_view(pages)?;

                    return Err(error);
                }

                Ok(pages)
            }
        }
    }
}
