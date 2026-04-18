use super::{
    LargeEntry, LargeEntryId, RawLocation, RawPointerEntry, RawSpace, SmallSpan, checked_pointer_id,
};
use crate::arena::{Arena, PageView, SpanSlot};
use crate::value::RawPointer;
use crate::{Bitmap, HeapError, HeapResult};

/// One raw payload source for entry initialization.
enum RawAllocationSource<'a> {
    /// One caller-provided byte payload.
    Bytes(&'a [u8]),
    /// One zeroed payload of the given byte length.
    Zeroed(usize),
}

impl RawAllocationSource<'_> {
    /// Return the logical byte length for this payload source.
    fn byte_len(&self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes.len(),
            Self::Zeroed(byte_len) => *byte_len,
        }
    }

    /// Initialize one raw small-slot payload.
    fn initialize_small_slot(
        &self,
        arena: &Arena,
        pages: &mut PageView,
        slot_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => arena.set_bytes(pages, slot_offset, bytes),
            Self::Zeroed(_) => Ok(()),
        }
    }
}

impl RawSpace {
    /// Return the projected mapped-byte delta for one raw entry.
    pub fn alloc_mapped_delta(&self, byte_len: usize) -> i64 {
        self.project_allocate_mapped_delta(byte_len)
    }

    /// Return the projected mapped-byte delta for one raw replacement.
    pub fn replace_mapped_delta(
        &self,
        pointer: RawPointer,
        next_byte_len: usize,
    ) -> HeapResult<i64> {
        // resolve the current live entry first
        let Some(record) = self.pointer(pointer).copied() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        // project the mapped-byte delta for this replacement
        let previous_mapped_bytes = self.location_mapped_bytes(location, record.byte_len);
        let next_mapped_bytes =
            self.location_replace_mapped_bytes(location, record.byte_len, next_byte_len);

        Ok(next_mapped_bytes as i64 - previous_mapped_bytes as i64)
    }

    /// Allocate one raw byte entry.
    pub fn allocate_bytes(&mut self, bytes: &[u8]) -> HeapResult<RawPointer> {
        self.allocate_with_source(RawAllocationSource::Bytes(bytes))
    }

    /// Allocate one zeroed raw byte entry.
    pub fn allocate_zeroed(&mut self, byte_len: usize) -> HeapResult<RawPointer> {
        self.allocate_with_source(RawAllocationSource::Zeroed(byte_len))
    }

    /// Allocate one raw payload from one explicit payload source.
    fn allocate_with_source(&mut self, source: RawAllocationSource<'_>) -> HeapResult<RawPointer> {
        // allocate the stable pointer id first
        let pointer_id = self.allocate_pointer_id()?;
        let pointer = RawPointer::new(pointer_id);
        let byte_len = source.byte_len();
        let location = self.allocate_location(source)?;

        // then install the live pointer record
        self.set_pointer_entry(pointer_id, RawPointerEntry::new(location, byte_len))?;

        // charge the live raw entry counters
        self.totals.allocate(byte_len, crate::HeapDomain::Raw)?;

        Ok(pointer)
    }

    /// Allocate one raw pointer id from the intrusive free list or the unused tail.
    fn allocate_pointer_id(&mut self) -> HeapResult<u32> {
        // reuse one freed pointer id when possible
        if let Some(pointer_id) = self.free_pointer_ids.pop() {
            checked_pointer_id(pointer_id)
        }
        // otherwise allocate from the unused tail
        else {
            let pointer_id = self.next_unused_pointer_id;
            let pointer_id = checked_pointer_id(pointer_id)?;
            self.next_unused_pointer_id = self.next_unused_pointer_id.checked_add(1).ok_or(
                HeapError::InvalidRawPointerId {
                    id: self.next_unused_pointer_id,
                },
            )?;

            Ok(pointer_id)
        }
    }
    /// Allocate one raw storage location for the given payload.
    fn allocate_location(&mut self, source: RawAllocationSource<'_>) -> HeapResult<RawLocation> {
        // prefer one small slot first
        if let Some(slot) = self.allocate_small(&source)? {
            return Ok(RawLocation::Small(slot));
        }

        // otherwise allocate one dedicated large-space entry
        let byte_len = source.byte_len();
        let pages = self.allocate_large_pages(source)?;
        let entry_id = self.store_large_entry(byte_len, pages)?;

        Ok(RawLocation::Large(entry_id))
    }

    /// Store one raw large entry in large space and return its stable id.
    pub(super) fn store_large_entry(
        &mut self,
        len: usize,
        pages: PageView,
    ) -> HeapResult<LargeEntryId> {
        // reuse one freed large entry id when possible
        let entry_id = if let Some(entry_id) = self.large.free_large_entry_ids.pop() {
            entry_id
        }
        // otherwise allocate from the unused tail
        else {
            let entry_id = self.large.next_unused_large_entry_id;
            let Some(next_entry_id) = self.large.next_unused_large_entry_id.checked_add(1) else {
                self.release_page_view(pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            self.large.next_unused_large_entry_id = next_entry_id;
            entry_id
        };

        if entry_id == 0 {
            self.release_page_view(pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        // materialize the stored entry record
        let entry = LargeEntry {
            is_live: true,
            len,
            pages,
        };
        let index = LargeEntryId::new(entry_id).index()?;

        // append at the unused tail when possible
        if let Err(error) = self.large.entries.set_or_push(index, entry) {
            self.release_page_view(pages)?;

            return Err(error);
        }

        Ok(LargeEntryId::new(entry_id))
    }

    /// Allocate one raw small slot if the payload fits one configured class.
    fn allocate_small(&mut self, source: &RawAllocationSource<'_>) -> HeapResult<Option<SpanSlot>> {
        let Some((class_index, span_index, slot_index)) =
            self.reserve_small_slot(source.byte_len())?
        else {
            return Ok(None);
        };

        self.initialize_small_slot(class_index, span_index, slot_index, source)
            .map(Some)
    }

    /// Allocate one raw small slot from one byte slice.
    pub(super) fn allocate_small_bytes(&mut self, bytes: &[u8]) -> HeapResult<Option<SpanSlot>> {
        self.allocate_small(&RawAllocationSource::Bytes(bytes))
    }

    /// Allocate or reuse one non-full raw span for the given size class.
    fn allocate_small_span(&mut self, class_index: usize, size_class: usize) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
                    let Some(span) = self.small.spans.get_mut(span_index) else {
                        self.release_page_view(pages)?;

                        return Err(HeapError::MissingSpan { span_index });
                    };

                    span.pages = pages;
                }

                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (self.small.span_bytes / size_class).max(1);
        let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
        let span = SmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            lengths: vec![0; slot_count].into_boxed_slice(),
            occupied: Bitmap::with_capacity(slot_count),
            pages,
        };
        let span_index = self.small.spans.len();
        if let Err(error) = self.small.spans.push(span) {
            self.release_page_view(pages)?;

            return Err(error);
        }

        Ok(span_index)
    }

    /// Reserve one raw small-slot location for the given byte length.
    fn reserve_small_slot(&mut self, byte_len: usize) -> HeapResult<Option<(usize, usize, usize)>> {
        // resolve the matching size class first
        let Some(class_index) = self.small.size_classes.class_index_for(byte_len) else {
            return Ok(None);
        };
        let size_class = self.small.size_classes.classes[class_index].bytes;
        let span_index = self.allocate_small_span(class_index, size_class)?;
        let Some(span) = self.small.spans.get(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_index = span.next_free_slot;

        Ok(Some((class_index, span_index, slot_index)))
    }

    /// Initialize one reserved raw small-slot location.
    fn initialize_small_slot(
        &mut self,
        class_index: usize,
        span_index: usize,
        slot_index: usize,
        source: &RawAllocationSource<'_>,
    ) -> HeapResult<SpanSlot> {
        // resolve the live span first
        let Some(span) = self.small.spans.get_mut(span_index) else {
            return Err(HeapError::MissingSpan { span_index });
        };
        let slot_offset =
            span.size_class
                .checked_mul(slot_index)
                .ok_or(HeapError::InvalidSmallSlot {
                    span_index,
                    slot_index,
                })?;

        // initialize the reserved slot payload
        source.initialize_small_slot(&self.arena, &mut span.pages, slot_offset)?;

        // mark the slot as live inside its span
        span.occupied.set(slot_index);
        span.lengths[slot_index] = source.byte_len();
        span.occupied_count =
            span.occupied_count
                .checked_add(1)
                .ok_or(HeapError::InvariantOverflow {
                    context: "raw span occupancy",
                })?;
        span.next_free_slot = span
            .occupied
            .first_clear_from(slot_index)
            .unwrap_or(span.slot_count);

        // requeue the span if it still has capacity
        if span.occupied_count < span.slot_count {
            self.small.available_spans[class_index].push(span_index);
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
            let size_class = span.size_class;

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
            span.lengths[slot_index] = 0;
            span.occupied_count -= 1;
            span.next_free_slot = span.next_free_slot.min(slot_index);

            // release fully empty span pages back into the local cache
            if span.occupied_count == 0 {
                span.next_free_slot = 0;

                let pages = span.pages;
                span.pages = PageView::empty();

                Some(pages)
            }
            // otherwise requeue the span once it transitions away from full
            else {
                let should_requeue = was_full && span.occupied_count < span.slot_count;
                if should_requeue
                    && let Some(class_index) = self.small.size_classes.class_index_for(size_class)
                {
                    self.small.available_spans[class_index].push(slot.span_index());
                }

                None
            }
        };

        if let Some(pages) = pages {
            self.release_page_view(pages)?;
        }

        Ok(())
    }

    /// Return the projected mapped-byte delta for one raw entry.
    fn project_allocate_mapped_delta(&self, byte_len: usize) -> i64 {
        // small entries only grow the heap when they need a fresh span
        if let Some(class_index) = self.small.size_classes.class_index_for(byte_len) {
            if self.has_available_small_slot(class_index) {
                return 0;
            }

            return self.small.span_bytes as i64;
        }

        self.round_up_large_entry_bytes(byte_len) as i64
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class_index: usize) -> bool {
        self.small.available_spans[class_index]
            .iter()
            .copied()
            .any(|span_index| {
                self.small
                    .spans
                    .get(span_index)
                    .map(|span| span.occupied_count < span.slot_count)
                    .unwrap_or(false)
            })
    }

    /// Return the mapped bytes currently charged to one raw location.
    fn location_mapped_bytes(&self, location: RawLocation, byte_len: usize) -> u64 {
        match location {
            RawLocation::Small(_) => 0,
            RawLocation::Large(_) => self.round_up_large_entry_bytes(byte_len),
        }
    }

    /// Return the mapped bytes for one replacement target location.
    fn location_replace_mapped_bytes(
        &self,
        location: RawLocation,
        previous_byte_len: usize,
        next_byte_len: usize,
    ) -> u64 {
        // small entries stay in place when the next payload still fits
        if let RawLocation::Small(slot) = location
            && let Some(span) = self.span(slot.span_index())
            && next_byte_len <= span.size_class
        {
            return 0;
        }

        // otherwise use the same mapped-byte delta model as fresh entry
        let _ = previous_byte_len;

        self.project_allocate_mapped_delta(next_byte_len) as u64
    }

    /// Return the page-rounded mapped bytes for one raw large entry.
    fn round_up_large_entry_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Allocate one dedicated large-entry page view for the given payload source.
    fn allocate_large_pages(&mut self, source: RawAllocationSource<'_>) -> HeapResult<PageView> {
        match source {
            RawAllocationSource::Bytes(bytes) => self.allocate_page_view_bytes(bytes),
            RawAllocationSource::Zeroed(byte_len) => self.allocate_page_view_zeroed(byte_len),
        }
    }
}
