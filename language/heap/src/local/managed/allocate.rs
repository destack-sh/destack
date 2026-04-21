use super::{
    CardSet, HeapScan, LargeEntry, LargeEntryId, ManagedLocation, ManagedReferenceEntry,
    ManagedSpace, ManagedYoungId, SmallSpan, YoungEntry, checked_packed_reference_id,
};
use crate::arena::{Arena, PageView, SpanAllocationPath, SpanSlot};
use crate::{Bitmap, HeapError, HeapResult, HeapSpace, LayoutId, ManagedReference, Shape, ShapeId};

/// One exact managed allocation path.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ManagedAllocationPath {
    /// One young-space allocation.
    Young,
    /// One small-space allocation.
    Small {
        /// The resolved size-class index.
        class_index: usize,
        /// The resolved slot width.
        size_class: usize,
        /// The mapped-byte delta for this path.
        mapped_delta: i64,
    },
    /// One large-space allocation.
    Large {
        /// The mapped-byte delta for this path.
        mapped_delta: i64,
    },
}

impl ManagedAllocationPath {
    /// Return the mapped-byte delta for this allocation path.
    pub(crate) fn mapped_delta(self) -> i64 {
        match self {
            Self::Young => 0,
            Self::Small { mapped_delta, .. } | Self::Large { mapped_delta } => mapped_delta,
        }
    }
}

impl ManagedAllocationPath {
    /// Build one mature managed allocation path from one shared span path.
    fn mature(path: SpanAllocationPath) -> Self {
        match path {
            SpanAllocationPath::Small {
                class_index,
                size_class,
                mapped_delta,
            } => Self::Small {
                class_index,
                size_class,
                mapped_delta,
            },
            SpanAllocationPath::Large { mapped_delta } => Self::Large { mapped_delta },
        }
    }
}

/// One managed slot initialization mode.
enum SlotWrite<'a> {
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

impl SlotWrite<'_> {
    /// Initialize one managed small-slot payload.
    fn initialize_slot(
        &self,
        arena: &Arena,
        pages: &mut PageView,
        slot_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => arena.set_bytes(pages, slot_offset, bytes),
            Self::Zeroed => Ok(()),
            Self::PageView {
                page_view,
                start,
                byte_len,
            } => arena.copy_bytes_between_page_views(
                page_view,
                *start,
                pages,
                slot_offset,
                *byte_len,
            ),
        }
    }
}

impl ManagedSpace {
    /// Return one exact managed allocation path for the requested byte length.
    pub(crate) fn allocation_path(&self, byte_len: usize) -> ManagedAllocationPath {
        // keep young allocations on the nursery tail when they still fit
        if byte_len <= self.max_young_allocation_bytes
            && self
                .young
                .next_offset
                .checked_add(byte_len)
                .is_some_and(|next_offset| next_offset <= self.young.capacity_bytes)
        {
            return ManagedAllocationPath::Young;
        }

        // otherwise resolve one mature span path
        let path = self.small.size_classes.span_allocation_path(
            byte_len,
            self.small.span_bytes,
            |class_index| self.has_available_small_slot(class_index),
            |large_bytes| self.round_up_large_entry_bytes(large_bytes),
        );

        ManagedAllocationPath::mature(path)
    }

    /// Allocate one managed byte entry.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        let path = self.allocation_path(bytes.len());

        self.allocate_with_bytes(bytes.len(), Some(bytes), scan, layout_id, path)
    }

    /// Allocate one zeroed managed byte entry.
    pub fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ManagedReference> {
        let path = self.allocation_path(byte_len);

        self.allocate_with_bytes(byte_len, None, scan, layout_id, path)
    }

    /// Allocate one managed byte entry through one precomputed allocation path.
    pub(crate) fn place_bytes(
        &mut self,
        bytes: &[u8],
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        path: ManagedAllocationPath,
    ) -> HeapResult<ManagedReference> {
        self.allocate_with_bytes(bytes.len(), Some(bytes), scan, layout_id, path)
    }

    /// Allocate one zeroed managed byte entry through one precomputed allocation path.
    pub(crate) fn place_zeroed(
        &mut self,
        byte_len: usize,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        path: ManagedAllocationPath,
    ) -> HeapResult<ManagedReference> {
        self.allocate_with_bytes(byte_len, None, scan, layout_id, path)
    }

    /// Free one managed entry.
    pub fn free(&mut self, reference: ManagedReference) -> HeapResult<bool> {
        // resolve the live entry first
        let reference_id = reference.id();
        let Some(record) = self.reference(reference).copied() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let freed_bytes = record.byte_len() as u64;
        self.usage.check_free(freed_bytes, HeapSpace::Managed)?;

        match location {
            // release one young entry in place
            ManagedLocation::Young(young_id) => {
                let Some(entry) = self.young_entry_mut(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                entry.is_live = false;

                // update heap usage
                self.usage.free(freed_bytes, HeapSpace::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                Ok(true)
            }

            // release one small-span slot
            ManagedLocation::Small(slot) => {
                self.release_small_slot(slot)?;

                // update heap usage
                self.usage.free(freed_bytes, HeapSpace::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                Ok(true)
            }

            // release one entry in large space and its arena pages
            ManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                let pages = entry.pages;

                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                // retire the live large-entry slot before releasing its pages
                entry.retire();
                self.large.free_large_entry_ids.push(entry_id.id());

                // update heap usage
                self.usage.free(freed_bytes, HeapSpace::Managed)?;

                // clear the stable reference slot
                self.retire_reference(reference_id)?;

                // release the old physical pages after the live slot is gone
                self.release_page_view(pages)?;

                Ok(true)
            }
        }
    }

    /// Allocate one managed payload from explicit bytes or one zeroed length.
    fn allocate_with_bytes(
        &mut self,
        byte_len: usize,
        bytes: Option<&[u8]>,
        scan: impl Into<HeapScan>,
        layout_id: Option<LayoutId>,
        path: ManagedAllocationPath,
    ) -> HeapResult<ManagedReference> {
        // resolve reference metadata and allocate one stable reference id
        let shape_id = self.shape_table.intern(Shape {
            scan: scan.into(),
            layout_id,
        })?;
        let reference_id = self.allocate_reference_id()?;
        let reference = ManagedReference::new(reference_id);
        let location = self.allocate_location(byte_len, bytes, shape_id, path)?;

        // install the live reference record and counters
        self.references.set_or_push(
            Self::reference_index(reference_id)?,
            ManagedReferenceEntry::new(location, byte_len),
        )?;
        self.usage.allocate(byte_len, HeapSpace::Managed)?;
        let Some(shape) = self.shape_table.shape(shape_id) else {
            return Err(HeapError::InvalidShapeId {
                index: shape_id.index(),
            });
        };

        // track every live reference whose shape may contain shared edges
        if shape.scan.has_shared_reference() {
            self.track_shared_edge_root(reference)?;
        }

        // queue newly published shared edges during an active shared cycle
        if bytes.is_some() {
            self.queue_shared_reference(reference)?;
        }

        Ok(reference)
    }

    /// Allocate one managed storage location for the given payload.
    fn allocate_location(
        &mut self,
        byte_len: usize,
        bytes: Option<&[u8]>,
        shape_id: ShapeId,
        path: ManagedAllocationPath,
    ) -> HeapResult<ManagedLocation> {
        match path {
            // execute the precomputed young path directly
            ManagedAllocationPath::Young => {
                let Some(young_id) = self.allocate_young(byte_len, bytes, shape_id)? else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: self.young.generation,
                        entry_index: self.young.entries.len() as u32,
                    });
                };

                Ok(ManagedLocation::Young(young_id))
            }

            // execute the precomputed small path directly
            ManagedAllocationPath::Small {
                class_index,
                size_class,
                ..
            } => {
                let init = match bytes {
                    Some(bytes) => SlotWrite::Bytes(bytes),
                    None => SlotWrite::Zeroed,
                };
                let span_index = self.allocate_small_span(class_index, size_class)?;
                let Some(span) = self.small.spans.get(span_index) else {
                    return Err(HeapError::MissingSpan { span_index });
                };
                let slot_index = span.next_free_slot;
                let slot = self.initialize_small_slot(
                    class_index,
                    span_index,
                    slot_index,
                    init,
                    byte_len,
                    shape_id,
                    true,
                )?;

                Ok(ManagedLocation::Small(slot))
            }

            // execute the precomputed large path directly
            ManagedAllocationPath::Large { .. } => {
                let pages = self.allocate_large_pages(byte_len, bytes)?;
                let entry_id = self.store_large_entry(byte_len, pages, shape_id, true)?;

                Ok(ManagedLocation::Large(entry_id))
            }
        }
    }

    /// Allocate one reference id from the intrusive free list or the unused tail.
    fn allocate_reference_id(&mut self) -> HeapResult<u32> {
        // reuse one freed reference id when possible
        if let Some(reference_id) = self.free_reference_ids.pop() {
            checked_packed_reference_id(reference_id)
        }
        // otherwise allocate from the unused tail
        else {
            let reference_id = self.next_unused_reference_id;
            let reference_id = checked_packed_reference_id(reference_id)?;
            self.next_unused_reference_id = self.next_unused_reference_id.checked_add(1).ok_or(
                HeapError::InvalidManagedReferenceId {
                    id: self.next_unused_reference_id,
                },
            )?;

            Ok(reference_id)
        }
    }

    /// Release one managed small slot without retiring its stable reference id.
    pub(crate) fn release_small_slot(&mut self, slot: SpanSlot) -> HeapResult<()> {
        let pages = {
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                });
            };

            let slot_index = slot.slot_index();
            let was_full = span.occupied_count == span.slot_count;
            let size_class = span.size_class;

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
            span.occupied_count -= 1;
            span.next_free_slot = span.next_free_slot.min(slot_index);
            span.set_shape_id(slot_index, None);

            // release fully empty span pages back into the local cache
            if span.occupied_count == 0 {
                span.next_free_slot = 0;
                span.dirty_cards.clear();
                span.is_dirty_queued = false;

                let pages = span.pages;
                span.pages = PageView::empty();

                Some(pages)
            }
            // otherwise requeue the span if it was full before the free
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

    /// Return whether one size class still has one live reusable slot.
    fn has_available_small_slot(&self, class_index: usize) -> bool {
        !self.small.available_spans[class_index].is_empty()
    }

    /// Return the page-rounded mapped bytes for one managed large entry.
    fn round_up_large_entry_bytes(&self, byte_len: usize) -> u64 {
        let page_bytes = self.large.page_bytes as u64;
        let byte_len = byte_len as u64;

        byte_len.div_ceil(page_bytes) * page_bytes
    }

    /// Store one large entry in large space and return its stable id.
    pub(crate) fn store_large_entry(
        &mut self,
        len: usize,
        pages: PageView,
        shape_id: ShapeId,
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
            pages,
            shape_id,
            dirty_cards: CardSet::with_len(len, self.card_bytes),
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
        bytes: Option<&[u8]>,
        shape_id: ShapeId,
    ) -> HeapResult<Option<ManagedYoungId>> {
        let Some((young_id, write_offset)) = self.reserve_young_entry(byte_len, shape_id) else {
            return Ok(None);
        };

        // initialize the reserved young-space range
        if let Some(bytes) = bytes {
            self.arena
                .set_bytes(&mut self.young.pages, write_offset, bytes)?;
        }

        Ok(Some(young_id))
    }

    /// Reserve one young entry slot when the young space has room.
    fn reserve_young_entry(
        &mut self,
        byte_len: usize,
        shape_id: ShapeId,
    ) -> Option<(ManagedYoungId, usize)> {
        // reject entries that do not fit the young-space tail
        let write_offset = self.young.next_offset;
        let end_offset = write_offset.checked_add(byte_len)?;
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
            shape_id,
            is_live: true,
        };

        // install the young-entry record
        self.young.entries.push(entry);

        // advance the young-space tail after installing the entry
        self.young.next_offset = end_offset;

        Some((
            ManagedYoungId::new(self.young.generation, entry_id),
            write_offset,
        ))
    }

    /// Allocate one copied small slot from one source page view.
    pub(crate) fn allocate_small_from_page_view(
        &mut self,
        source_page_view: &PageView,
        source_start: usize,
        byte_len: usize,
        shape_id: ShapeId,
        remember: bool,
    ) -> HeapResult<Option<SpanSlot>> {
        let Some((class_index, span_index, slot_index)) = self.reserve_small_slot(byte_len)? else {
            return Ok(None);
        };

        self.initialize_small_slot(
            class_index,
            span_index,
            slot_index,
            SlotWrite::PageView {
                page_view: source_page_view,
                start: source_start,
                byte_len,
            },
            byte_len,
            shape_id,
            remember,
        )
        .map(Some)
    }

    /// Allocate one dedicated large-entry page view for the given payload source.
    fn allocate_large_pages(
        &mut self,
        byte_len: usize,
        bytes: Option<&[u8]>,
    ) -> HeapResult<PageView> {
        match bytes {
            Some(bytes) => self.allocate_page_view_bytes(bytes),
            None => self.allocate_page_view_zeroed(byte_len),
        }
    }

    /// Reserve one managed small-slot location for the given byte length.
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

    /// Allocate or reuse one non-full managed span for the given size class.
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
        let dirty_card_bytes =
            slot_count
                .checked_mul(size_class)
                .ok_or(HeapError::InvariantOverflow {
                    context: "managed span dirty-card bytes",
                })?;
        let pages = self.allocate_page_view_zeroed(self.small.span_bytes)?;
        let span = SmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: Bitmap::with_capacity(slot_count),
            shape_ids: vec![None; slot_count].into_boxed_slice(),
            pages,
            dirty_cards: CardSet::with_len(dirty_card_bytes, self.card_bytes),
            is_dirty_queued: false,
        };
        let span_index = self.small.spans.len();
        if let Err(error) = self.small.spans.push(span) {
            self.release_page_view(pages)?;

            return Err(error);
        }

        Ok(span_index)
    }

    /// Initialize one reserved managed small-slot location.
    fn initialize_small_slot(
        &mut self,
        class_index: usize,
        span_index: usize,
        slot_index: usize,
        init: SlotWrite<'_>,
        byte_len: usize,
        shape_id: ShapeId,
        remember: bool,
    ) -> HeapResult<SpanSlot> {
        let should_requeue;
        let init_error;

        // initialize and install the reserved slot while the span is borrowed
        if let Some(span) = self.small.spans.get_mut(span_index) {
            let slot_offset =
                span.size_class
                    .checked_mul(slot_index)
                    .ok_or(HeapError::InvalidSmallSlot {
                        span_index,
                        slot_index,
                    })?;

            if let Err(error) = init.initialize_slot(&self.arena, &mut span.pages, slot_offset) {
                should_requeue = span.occupied_count < span.slot_count;
                init_error = Some(error);
            } else {
                span.occupied.set(slot_index);
                span.occupied_count =
                    span.occupied_count
                        .checked_add(1)
                        .ok_or(HeapError::InvariantOverflow {
                            context: "managed span occupancy",
                        })?;
                span.next_free_slot = span
                    .occupied
                    .first_clear_from(slot_index)
                    .unwrap_or(span.slot_count);
                span.set_shape_id(slot_index, Some(shape_id));
                should_requeue = span.occupied_count < span.slot_count;
                init_error = None;
            }
        } else {
            return Err(HeapError::MissingSpan { span_index });
        }

        // requeue the reserved span when it still has capacity
        if should_requeue {
            self.small.available_spans[class_index].push(span_index);
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
