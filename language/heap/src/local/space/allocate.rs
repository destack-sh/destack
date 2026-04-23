use destack_mir::{LayoutId, ReferenceMap};

use super::{
    CardSet, HeapPageOwner, HeapSpace, HeapStorage, HeapYoungId, LargeEntry, LargeEntryId,
    SmallSpan, YoungEntry,
};
use crate::allocator::{Allocator, PageView, SpanSlot};
use crate::{
    AccountingRegion, Allocation, Bitmap, HeapError, HeapReference, HeapResult, SmallSpanClass,
    clear_slot_reference_bits, write_slot_reference_bits,
};

/// One heap slot initialization mode.
enum SlotInit<'a> {
    /// One caller-provided byte payload.
    Bytes(&'a [u8]),
    /// One zeroed payload.
    Zeroed,
    /// One logical byte range copied from one page view.
    PageView {
        /// The source logical page view.
        page_view: &'a PageView,
        /// The source byte offset.
        start: usize,
        /// The copied byte length.
        byte_len: usize,
    },
}

impl SlotInit<'_> {
    /// Initialize one heap small-slot payload.
    fn initialize_slot(
        &self,
        allocator: &Allocator,
        pages: &mut PageView,
        slot_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => allocator.set_bytes(pages, slot_offset, bytes),
            Self::Zeroed => Ok(()),
            Self::PageView {
                page_view,
                start,
                byte_len,
            } => allocator.copy_bytes_between_page_views(
                page_view,
                *start,
                pages,
                slot_offset,
                *byte_len,
            ),
        }
    }
}

impl HeapSpace {
    /// Return the projected mapped-byte delta for one typed heap allocation.
    pub(crate) fn mapped_byte_delta(&self, layout_id: LayoutId) -> HeapResult<i64> {
        let byte_len = self.layout_byte_len(layout_id)?;
        let reference_map = self.reference_map(layout_id)?;

        // stay in young space when the payload still fits
        if self.young_fits(byte_len) {
            return Ok(0);
        }

        // use one traced small span when the payload still fits
        if let Some(class) = self.small_span_class(byte_len, reference_map) {
            if self.has_available_small_slot(&class)? {
                return Ok(0);
            }

            return Ok(self.small.span_bytes as i64);
        }

        // otherwise fall back to one dedicated large entry
        Ok(self.round_up_large_entry_bytes(byte_len) as i64)
    }

    /// Allocate one managed heap entry.
    pub fn allocate(
        &mut self,
        layout_id: LayoutId,
        allocation: Allocation<'_>,
    ) -> HeapResult<HeapReference> {
        let byte_len = self.layout_byte_len(layout_id)?;
        let has_shared_reference = self.reference_map(layout_id)?.has_shared_reference();
        let storage = self.allocate_storage(layout_id, byte_len, allocation)?;
        let reference = self.base_reference(storage)?;

        self.usage.allocate(byte_len, AccountingRegion::Heap)?;

        // track every live reference whose shape may contain shared edges
        if has_shared_reference {
            self.track_shared_edge_root(reference)?;
        }

        // queue newly published shared edges during an active shared cycle
        if allocation.bytes().is_some() {
            self.queue_shared_reference(reference)?;
        }

        Ok(reference)
    }

    /// Free one heap entry.
    pub fn free(&mut self, reference: HeapReference) -> HeapResult<bool> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let freed_bytes = location.byte_len as u64;
        self.usage.check_free(freed_bytes, AccountingRegion::Heap)?;

        match location.storage {
            // release one young entry in place
            HeapStorage::Young(young_id) => {
                let Some(entry) = self.young_entry_mut(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                entry.is_live = false;
                entry.is_marked = false;

                self.usage.free(freed_bytes, AccountingRegion::Heap)?;
                self.remove_shared_edge_root(reference)?;

                Ok(true)
            }

            // release one small-span slot
            HeapStorage::Small(slot) => {
                self.release_small_slot(slot)?;
                self.usage.free(freed_bytes, AccountingRegion::Heap)?;
                self.remove_shared_edge_root(reference)?;

                Ok(true)
            }

            // release one entry in large space and its allocator pages
            HeapStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                let pages = entry.pages.clone();

                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                // retire the live large-entry slot before releasing its pages
                entry.retire();
                self.large.free_large_entry_ids.push(entry_id.id());

                self.usage.free(freed_bytes, AccountingRegion::Heap)?;
                self.remove_shared_edge_root(reference)?;

                self.unmap_page_view(&pages)?;
                self.release_page_view(pages)?;

                Ok(true)
            }
        }
    }

    /// Allocate one heap storage partition for the given payload.
    fn allocate_storage(
        &mut self,
        layout_id: LayoutId,
        byte_len: usize,
        allocation: Allocation<'_>,
    ) -> HeapResult<HeapStorage> {
        let reference_map = self.reference_map(layout_id)?.clone();

        // reject inconsistent allocation
        if let Some(bytes) = allocation.bytes()
            && bytes.len() != byte_len
        {
            return Err(HeapError::InvalidLayoutBytes {
                layout_id,
                expected: byte_len,
                actual: bytes.len(),
            });
        }

        // allocate into young space when the payload still fits there
        if self.young_fits(byte_len) {
            let Some(young_id) = self.allocate_young(byte_len, allocation, layout_id)? else {
                return Err(HeapError::MissingYoungEntry {
                    generation: self.young.generation,
                    entry_index: self.young.entries.len() as u32,
                });
            };

            Ok(HeapStorage::Young(young_id))
        }
        // otherwise allocate from one homogeneous small span when the payload still fits
        else if let Some(class) = self.small_span_class(byte_len, &reference_map) {
            let init = match allocation {
                Allocation::Bytes(bytes) => SlotInit::Bytes(bytes),
                Allocation::Zeroed => SlotInit::Zeroed,
            };
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
                &reference_map,
                init,
                true,
            )?;

            Ok(HeapStorage::Small(slot))
        }
        // otherwise allocate one dedicated large entry
        else {
            let pages = self.allocate_large_pages(byte_len, allocation)?;
            let entry_id = self.store_large_entry(byte_len, pages, layout_id, true)?;

            Ok(HeapStorage::Large(entry_id))
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
            span.byte_lens[slot_index] = 0;
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

                let pages = span.pages.clone();
                span.pages = PageView::empty();

                Some(pages)
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
            self.small.available_spans[bucket_index].push(slot.span_index());
        }

        if let Some(pages) = pages {
            self.unmap_page_view(&pages)?;
            self.release_page_view(pages)?;
        }

        Ok(())
    }

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class: &SmallSpanClass) -> HeapResult<bool> {
        let bucket_index = self.small_span_bucket(class)?;

        Ok(self
            .small
            .available_spans
            .get(bucket_index)
            .is_some_and(|spans| !spans.is_empty()))
    }

    /// Report whether one byte length still fits the young-space tail.
    fn young_fits(&self, byte_len: usize) -> bool {
        byte_len <= self.max_young_allocation_bytes
            && self
                .young
                .next_offset
                .checked_add(byte_len)
                .is_some_and(|next_offset| next_offset <= self.young.capacity_bytes)
    }

    /// Return one homogeneous small-span class for the given layout when it fits.
    fn small_span_class(
        &self,
        byte_len: usize,
        reference_map: &ReferenceMap,
    ) -> Option<SmallSpanClass> {
        let class_index = self.small.size_classes.class_index_for(byte_len)?;

        Some(SmallSpanClass {
            size_class: self.small.size_classes.classes[class_index].bytes,
            is_noscan: !reference_map.has_reference(),
        })
    }

    /// Return the page-rounded mapped bytes for one heap large entry.
    fn round_up_large_entry_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Store one large entry in large space and return its entry id.
    pub(crate) fn store_large_entry(
        &mut self,
        len: usize,
        pages: PageView,
        layout_id: LayoutId,
        remember: bool,
    ) -> HeapResult<LargeEntryId> {
        // reuse one freed large entry id when possible
        let (entry_id, reused_entry_id) = if let Some(entry_id) =
            self.large.free_large_entry_ids.pop()
        {
            (entry_id, true)
        }
        // otherwise allocate from the unused tail
        else {
            let entry_id = self.large.next_unused_large_entry_id;
            let Some(next_entry_id) = self.large.next_unused_large_entry_id.checked_add(1) else {
                self.release_page_view(pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            self.large.next_unused_large_entry_id = next_entry_id;
            (entry_id, false)
        };

        if entry_id == 0 {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.release_page_view(pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        // materialize the stored entry record
        let entry = LargeEntry {
            is_live: true,
            len,
            pages: pages.clone(),
            layout_id,
            is_marked: false,
            dirty_cards: CardSet::with_len(len),
            is_dirty_queued: false,
        };
        let index = LargeEntryId::new(entry_id).index()?;

        if let Err(error) = self.large.entries.set_or_push(index, entry) {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.release_page_view(pages)?;

            return Err(error);
        }

        let entry_id = LargeEntryId::new(entry_id);

        self.map_page_view(&pages, |logical_page_index| HeapPageOwner::Large {
            entry_id,
            logical_page_index,
        })?;

        // remember new mature entries conservatively
        if remember {
            self.mark_large_entry_dirty(entry_id, 0, len)?;
        }

        Ok(entry_id)
    }

    /// Allocate one young entry if the young space has room.
    fn allocate_young(
        &mut self,
        byte_len: usize,
        allocation: Allocation<'_>,
        layout_id: LayoutId,
    ) -> HeapResult<Option<HeapYoungId>> {
        let Some((young_id, write_offset)) = self.reserve_young_entry(byte_len, layout_id) else {
            return Ok(None);
        };

        // initialize the reserved young-space range
        if let Allocation::Bytes(bytes) = allocation {
            self.allocator
                .set_bytes(&mut self.young.pages, write_offset, bytes)?;
        }

        Ok(Some(young_id))
    }

    /// Reserve one young entry slot when the young space has room.
    fn reserve_young_entry(
        &mut self,
        byte_len: usize,
        layout_id: LayoutId,
    ) -> Option<(HeapYoungId, usize)> {
        // zero-byte entries still need one distinct address
        let storage_byte_len = byte_len.max(1);

        // reject entries that do not fit the young-space tail
        let write_offset = self.young.next_offset;
        let end_offset = write_offset.checked_add(storage_byte_len)?;
        if end_offset > self.young.capacity_bytes {
            return None;
        }

        // append one new young-entry record at the tail
        let entry_id = self.young.entries.len() as u32;

        // materialize the young-entry record
        let entry = YoungEntry {
            first_page: (write_offset / self.young.page_bytes) as u32,
            first_offset: (write_offset % self.young.page_bytes) as u32,
            byte_len,
            layout_id,
            is_live: true,
            is_marked: false,
        };

        // install the young-entry record
        self.young.entries.push(entry);

        // advance the young-space tail after installing the entry
        self.young.next_offset = end_offset;

        Some((
            HeapYoungId::new(self.young.generation, entry_id),
            write_offset,
        ))
    }

    /// Allocate one copied small slot from one source page view.
    pub(crate) fn allocate_small_from_page_view(
        &mut self,
        source_page_view: &PageView,
        source_start: usize,
        layout_id: LayoutId,
        remember: bool,
    ) -> HeapResult<Option<SpanSlot>> {
        let Some((class, span_index, slot_index)) = self.reserve_small_slot(layout_id)? else {
            return Ok(None);
        };
        let byte_len = self.layout_byte_len(layout_id)?;
        let reference_map = self.reference_map(layout_id)?.clone();

        self.initialize_small_slot(
            &class,
            span_index,
            slot_index,
            byte_len,
            &reference_map,
            SlotInit::PageView {
                page_view: source_page_view,
                start: source_start,
                byte_len,
            },
            remember,
        )
        .map(Some)
    }

    /// Allocate one dedicated large-entry page view for the given payload source.
    fn allocate_large_pages(
        &mut self,
        byte_len: usize,
        allocation: Allocation<'_>,
    ) -> HeapResult<PageView> {
        match allocation {
            Allocation::Bytes(bytes) => self.allocate_page_view_bytes(bytes),
            Allocation::Zeroed => self.allocate_page_view_zeroed(byte_len),
        }
    }

    /// Reserve one heap small-slot location for the given byte length.
    fn reserve_small_slot(
        &mut self,
        layout_id: LayoutId,
    ) -> HeapResult<Option<(SmallSpanClass, usize, usize)>> {
        let byte_len = self.layout_byte_len(layout_id)?;
        let reference_map = self.reference_map(layout_id)?.clone();

        // resolve the matching size class first
        let Some(class) = self.small_span_class(byte_len, &reference_map) else {
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
        while let Some(span_index) = self.small.available_spans[bucket_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                if span.occupied_count == 0 && span.pages.is_empty() {
                    let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
                    self.map_page_view(&pages, |logical_page_index| HeapPageOwner::Small {
                        span_index,
                        logical_page_index,
                    })?;

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
        let scan_word_count = class.size_class.div_ceil(std::mem::size_of::<usize>());
        let dirty_card_bytes =
            slot_count
                .checked_mul(class.size_class)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap span dirty-card bytes",
                })?;
        let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
        let span = SmallSpan {
            class: class.clone(),
            slot_count,
            byte_lens: vec![0; slot_count].into_boxed_slice(),
            occupied_count: 0,
            free_cursor: 0,
            occupied: Bitmap::with_capacity(slot_count),
            local_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            shared_reference_bits: Bitmap::with_capacity(slot_count * scan_word_count),
            marked: Bitmap::with_capacity(slot_count),
            pages: pages.clone(),
            dirty_cards: CardSet::with_len(dirty_card_bytes),
            is_dirty_queued: false,
        };
        let span_index = self.small.spans.len();
        if let Err(error) = self.small.spans.push(span) {
            self.release_page_view(pages)?;

            return Err(error);
        }

        self.map_page_view(&pages, |logical_page_index| HeapPageOwner::Small {
            span_index,
            logical_page_index,
        })?;

        Ok(span_index)
    }

    /// Initialize one reserved heap small-slot location.
    fn initialize_small_slot(
        &mut self,
        class: &SmallSpanClass,
        span_index: usize,
        slot_index: usize,
        byte_len: usize,
        reference_map: &ReferenceMap,
        init: SlotInit<'_>,
        remember: bool,
    ) -> HeapResult<SpanSlot> {
        let should_requeue;
        let init_error;

        // initialize and install the reserved slot while the span is borrowed
        if let Some(span) = self.small.spans.get_mut(span_index) {
            let slot_offset = span.class.size_class.checked_mul(slot_index).ok_or(
                HeapError::InvalidSmallSlot {
                    span_index,
                    slot_index,
                },
            )?;

            if let Err(error) = init.initialize_slot(&self.allocator, &mut span.pages, slot_offset)
            {
                should_requeue = span.occupied_count < span.slot_count;
                init_error = Some(error);
            } else {
                span.occupied.set(slot_index);
                span.marked.clear(slot_index);
                span.byte_lens[slot_index] = byte_len;
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
                span.occupied_count =
                    span.occupied_count
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "heap span occupancy",
                        })?;
                span.free_cursor = span
                    .occupied
                    .first_clear_from(slot_index)
                    .unwrap_or(span.slot_count);
                should_requeue = span.occupied_count < span.slot_count;
                init_error = None;
            }
        } else {
            return Err(HeapError::MissingSpan { span_index });
        }

        // requeue the reserved span when it still has capacity
        if should_requeue {
            let bucket_index = self.small_span_bucket(class)?;
            self.small.available_spans[bucket_index].push(span_index);
        }

        if let Some(error) = init_error {
            return Err(error);
        }

        let slot = SpanSlot::new(span_index, slot_index)?;

        // remember new mature entries conservatively
        if remember {
            self.mark_span_slot_dirty(span_index, slot_index, 0, byte_len)?;
        }

        Ok(slot)
    }
}
