use super::{
    LargeEntry, LargeEntryId, ManagedLocation, ManagedReferenceEntry, ManagedSpace, ManagedYoungId,
    MapId, ReferenceMap, SmallSpan, YoungEntry, checked_reference_id,
};
use crate::alloc::{Arena, PageView, SpanSlot};
use crate::gc::CardSet;
use crate::value::ManagedReference;
use crate::{Bitmap, HeapDomain, HeapError, HeapResult, StorageLayoutId};

/// One managed payload source for entry initialization.
enum ManagedAllocationSource<'a> {
    /// One caller-provided byte payload.
    Bytes(&'a [u8]),
    /// One zeroed payload of the given byte length.
    Zeroed(usize),
}

impl ManagedAllocationSource<'_> {
    /// Return the logical byte length for this payload source.
    fn byte_len(&self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes.len(),
            Self::Zeroed(byte_len) => *byte_len,
        }
    }

    /// Return one small-slot source for this payload.
    fn as_small_source(&self) -> ManagedSmallSource<'_> {
        match self {
            Self::Bytes(bytes) => ManagedSmallSource::Bytes(bytes),
            Self::Zeroed(byte_len) => ManagedSmallSource::Zeroed(*byte_len),
        }
    }

    /// Write this payload into one young-space byte range.
    fn write_young(
        &self,
        arena: &Arena,
        pages: &mut PageView,
        write_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => arena.set_bytes(pages, write_offset, bytes),
            Self::Zeroed(_) => Ok(()),
        }
    }

    /// Materialize one dedicated large-entry page view for this payload.
    fn allocate_large_pages(self, arena: &Arena) -> HeapResult<PageView> {
        match self {
            Self::Bytes(bytes) => arena.allocate_bytes(bytes),
            Self::Zeroed(byte_len) => arena.allocate_zeroed(byte_len),
        }
    }
}

/// One managed small-slot initialization source.
enum ManagedSmallSource<'a> {
    /// One caller-provided byte payload.
    Bytes(&'a [u8]),
    /// One zeroed payload of the given byte length.
    Zeroed(usize),
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

impl ManagedSmallSource<'_> {
    /// Return the logical byte length for this slot source.
    fn byte_len(&self) -> usize {
        match self {
            Self::Bytes(bytes) => bytes.len(),
            Self::Zeroed(byte_len) => *byte_len,
            Self::PageView { byte_len, .. } => *byte_len,
        }
    }

    /// Initialize one managed small-slot payload.
    fn initialize_slot(
        &self,
        arena: &Arena,
        pages: &mut PageView,
        slot_offset: usize,
    ) -> HeapResult<()> {
        match self {
            Self::Bytes(bytes) => arena.set_bytes(pages, slot_offset, bytes),
            Self::Zeroed(_) => Ok(()),
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
    /// Return the projected mapped-byte delta for one managed entry.
    pub fn alloc_mapped_delta(
        &self,
        byte_len: usize,
        _reference_map: &ReferenceMap,
        _layout_id: Option<StorageLayoutId>,
    ) -> i64 {
        self.project_allocate_mapped_delta(byte_len)
    }

    /// Allocate one managed byte entry.
    pub fn allocate_bytes(
        &mut self,
        bytes: &[u8],
        reference_map: ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        self.allocate_with_source(
            ManagedAllocationSource::Bytes(bytes),
            reference_map,
            layout_id,
        )
    }

    /// Allocate one zeroed managed byte entry.
    pub fn allocate_zeroed(
        &mut self,
        byte_len: usize,
        reference_map: ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        self.allocate_with_source(
            ManagedAllocationSource::Zeroed(byte_len),
            reference_map,
            layout_id,
        )
    }

    /// Allocate one managed payload from one explicit payload source.
    fn allocate_with_source(
        &mut self,
        source: ManagedAllocationSource<'_>,
        reference_map: ReferenceMap,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedReference> {
        // resolve reference metadata and allocate one stable reference id
        let (map_id, _) = self.map_table.intern(reference_map)?;
        let reference_id = self.allocate_reference_id()?;
        let reference = ManagedReference::new(reference_id);
        let byte_len = source.byte_len();
        let location = self.allocate_location(source, map_id, layout_id)?;

        // install the live reference record and counters
        self.set_reference_entry(reference_id, ManagedReferenceEntry::new(location, byte_len))?;
        self.totals.allocate(byte_len, HeapDomain::Managed)?;

        Ok(reference)
    }

    /// Allocate one managed storage location for the given payload.
    fn allocate_location(
        &mut self,
        source: ManagedAllocationSource<'_>,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<ManagedLocation> {
        let byte_len = source.byte_len();

        // prefer young space first
        if byte_len <= self.max_young_allocation_bytes
            && let Some(young_id) = self.allocate_young(&source, map_id, layout_id)?
        {
            return Ok(ManagedLocation::Young(young_id));
        }

        // otherwise try one mature small slot
        if let Some(slot) =
            self.allocate_small(source.as_small_source(), map_id, layout_id, true)?
        {
            return Ok(ManagedLocation::Small(slot));
        }

        // otherwise fall back to one dedicated large entry
        let pages = source.allocate_large_pages(&self.arena)?;
        let entry_id = self.store_large_entry(byte_len, pages, map_id, layout_id, true)?;

        Ok(ManagedLocation::Large(entry_id))
    }

    /// Allocate one reference id from the intrusive free list or the unused tail.
    fn allocate_reference_id(&mut self) -> HeapResult<u32> {
        // reuse one freed reference id when possible
        if let Some(reference_id) = self.free_reference_ids.pop() {
            checked_reference_id(reference_id)
        }
        // otherwise allocate from the unused tail
        else {
            let reference_id = self.next_unused_reference_id;
            let reference_id = checked_reference_id(reference_id)?;
            self.next_unused_reference_id = self.next_unused_reference_id.checked_add(1).ok_or(
                HeapError::InvalidManagedReferenceId {
                    id: self.next_unused_reference_id,
                },
            )?;

            Ok(reference_id)
        }
    }
    /// Return the projected mapped-byte delta for one managed entry.
    fn project_allocate_mapped_delta(&self, byte_len: usize) -> i64 {
        // young entries only grow mapped usage when they spill past the tail
        if byte_len <= self.max_young_allocation_bytes
            && self
                .young
                .next_offset
                .checked_add(byte_len)
                .is_some_and(|next_offset| next_offset <= self.young.capacity_bytes)
        {
            return 0;
        }

        // small entries only grow mapped usage when they need a fresh span
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
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
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
                self.arena.release_page_view(&pages)?;

                return Err(HeapError::InvalidLargeEntryId { id: entry_id });
            };

            self.large.next_unused_large_entry_id = next_entry_id;
            (entry_id, false)
        };

        if entry_id == 0 {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.arena.release_page_view(&pages)?;

            return Err(HeapError::InvalidLargeEntryId { id: entry_id });
        }

        // materialize the stored entry record
        let entry = LargeEntry {
            is_live: true,
            len,
            pages,
            map_id,
            layout_id,
            dirty_cards: CardSet::with_len(len, self.card_bytes),
            is_dirty_queued: false,
        };
        let index = LargeEntryId::new(entry_id).index()?;

        if let Err(error) = self.large.entries.set_or_push(index, entry) {
            if reused_entry_id {
                self.large.free_large_entry_ids.push(entry_id);
            }

            self.arena.release_page_view(&pages)?;

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
        source: &ManagedAllocationSource<'_>,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
    ) -> HeapResult<Option<ManagedYoungId>> {
        let Some((young_id, write_offset)) =
            self.reserve_young_entry(source.byte_len(), map_id, layout_id)
        else {
            return Ok(None);
        };

        // initialize the reserved young-space range
        source.write_young(&self.arena, &mut self.young.pages, write_offset)?;

        Ok(Some(young_id))
    }

    /// Reserve one young entry slot when the young space has room.
    fn reserve_young_entry(
        &mut self,
        byte_len: usize,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
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
            map_id,
            layout_id,
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

    /// Allocate one small managed slot from one explicit initialization source.
    fn allocate_small(
        &mut self,
        source: ManagedSmallSource<'_>,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
        remember: bool,
    ) -> HeapResult<Option<SpanSlot>> {
        let Some((class_index, span_index, slot_index)) =
            self.reserve_small_slot(source.byte_len())?
        else {
            return Ok(None);
        };

        self.initialize_small_slot(
            class_index,
            span_index,
            slot_index,
            source,
            map_id,
            layout_id,
            remember,
        )
        .map(Some)
    }

    /// Allocate one copied small slot from one source page view.
    pub(crate) fn allocate_small_from_page_view(
        &mut self,
        source_page_view: &PageView,
        source_start: usize,
        byte_len: usize,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
        remember: bool,
    ) -> HeapResult<Option<SpanSlot>> {
        self.allocate_small(
            ManagedSmallSource::PageView {
                page_view: source_page_view,
                start: source_start,
                byte_len,
            },
            map_id,
            layout_id,
            remember,
        )
    }

    /// Allocate or reuse one non-full managed span for the given size class.
    fn allocate_small_span(&mut self, class_index: usize, size_class: usize) -> HeapResult<usize> {
        // reuse one non-full span when possible
        while let Some(span_index) = self.small.available_spans[class_index].pop() {
            let Some(span) = self.small.spans.get(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            if span.occupied_count < span.slot_count {
                return Ok(span_index);
            }
        }

        // otherwise allocate one fresh span for the size class
        let slot_count = (self.small.span_bytes / size_class).max(1);
        let dirty_card_bytes =
            slot_count
                .checked_mul(size_class)
                .ok_or(HeapError::CountOverflow {
                    current: slot_count,
                    added: size_class,
                })?;
        let span = SmallSpan {
            size_class,
            slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: Bitmap::with_capacity(slot_count),
            map_ids: vec![MapId::empty(); slot_count].into_boxed_slice(),
            layout_ids: vec![None; slot_count].into_boxed_slice(),
            pages: self.arena.allocate_zeroed(self.small.span_bytes)?,
            dirty_cards: CardSet::with_len(dirty_card_bytes, self.card_bytes),
            is_dirty_queued: false,
        };
        let span_index = self.small.spans.len();
        self.small.spans.push(span)?;

        Ok(span_index)
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

    /// Initialize one reserved managed small-slot location.
    fn initialize_small_slot(
        &mut self,
        class_index: usize,
        span_index: usize,
        slot_index: usize,
        source: ManagedSmallSource<'_>,
        map_id: MapId,
        layout_id: Option<StorageLayoutId>,
        remember: bool,
    ) -> HeapResult<SpanSlot> {
        let source_byte_len = source.byte_len();
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

            if let Err(error) = source.initialize_slot(&self.arena, &mut span.pages, slot_offset) {
                should_requeue = span.occupied_count < span.slot_count;
                init_error = Some(error);
            } else {
                span.occupied.set(slot_index);
                span.occupied_count =
                    span.occupied_count
                        .checked_add(1)
                        .ok_or(HeapError::CountOverflow {
                            current: span.occupied_count,
                            added: 1,
                        })?;
                span.next_free_slot = span
                    .occupied
                    .first_clear_from(slot_index)
                    .unwrap_or(span.slot_count);
                span.set_map_id(slot_index, map_id);
                span.set_layout_id(slot_index, layout_id);
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
            self.mark_span_slot_dirty(span_index, slot_index, 0, source_byte_len)?;
        }

        Ok(slot)
    }
}
