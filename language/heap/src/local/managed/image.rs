use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    CardSet, EdgeId, EdgeMap, EdgeTable, GcState, LargeEntry, LargeEntryImage,
    ManagedReferenceEntry, ManagedSpace, MarkSet, SmallSpan, SmallSpanImage, TraceQueue,
    YoungImage, YoungSpace,
};
use crate::arena::{Arena, PageRunCache, PageView, SizeClassTable};
use crate::{AllocationTotals, CowTable, HeapError, HeapOptions, HeapResult};

/// One frozen managed-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ManagedSpaceImage {
    /// The captured branchable young-space root.
    young: YoungImage,

    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured managed spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured local page width.
    page_bytes: usize,
    /// The captured managed entries in large space.
    entries: Box<[LargeEntryImage]>,

    /// Dense managed reference metadata keyed by reference id minus one.
    references: Box<[ManagedReferenceEntry]>,
    /// The captured edge maps.
    edge_maps: Box<[EdgeMap]>,

    /// The encoded byte width for managed references inside traced payloads.
    managed_reference_bytes: u8,
    /// The configured young-space byte width.
    young_bytes: usize,
    /// The configured maximum payload size admitted into young space.
    max_young_allocation_bytes: usize,
    /// The configured remembered-card width.
    card_bytes: usize,
    /// The configured entry count per metadata table chunk.
    table_chunk_len: usize,

    /// The next managed reference id to allocate.
    next_unused_reference_id: u64,
    /// The next managed entry id to allocate in large space.
    next_unused_large_entry_id: u64,
    /// The number of allocated managed references.
    allocated_count: usize,
    /// The number of allocated managed bytes.
    allocated_bytes: u64,

    /// The captured GC state.
    gc_state: GcState,
}

#[allow(clippy::too_many_arguments)]
impl ManagedSpaceImage {
    /// Create one frozen managed-space root.
    pub(crate) fn new(
        young: YoungImage,
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_bytes: usize,
        entries: Box<[LargeEntryImage]>,
        references: Box<[ManagedReferenceEntry]>,
        edge_maps: Box<[EdgeMap]>,
        managed_reference_bytes: u8,
        young_bytes: usize,
        max_young_allocation_bytes: usize,
        card_bytes: usize,
        table_chunk_len: usize,
        next_unused_reference_id: u64,
        next_unused_large_entry_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            young,
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            entries,
            references,
            edge_maps,
            managed_reference_bytes,
            young_bytes,
            max_young_allocation_bytes,
            card_bytes,
            table_chunk_len,
            next_unused_reference_id,
            next_unused_large_entry_id,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the branchable young-space root.
    pub(crate) fn young(&self) -> &YoungImage {
        &self.young
    }

    /// Return the configured size-class table.
    pub(crate) fn size_classes(&self) -> &SizeClassTable {
        &self.size_classes
    }

    /// Return the configured small-space span width.
    pub(crate) const fn small_bytes(&self) -> usize {
        self.small_bytes
    }

    /// Return the captured managed spans.
    pub(crate) fn spans(&self) -> &[SmallSpanImage] {
        &self.spans
    }

    /// Return the configured local page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured managed entries in large space.
    pub(crate) fn entries(&self) -> &[LargeEntryImage] {
        &self.entries
    }

    /// Return the captured managed reference table.
    pub(crate) fn references(&self) -> &[ManagedReferenceEntry] {
        &self.references
    }

    /// Return the captured edge maps.
    pub(crate) fn edge_maps(&self) -> &[EdgeMap] {
        &self.edge_maps
    }

    /// Return the encoded byte width for managed references.
    pub(crate) const fn managed_reference_bytes(&self) -> u8 {
        self.managed_reference_bytes
    }

    #[cfg(test)]
    /// Return this image with one explicit managed-reference width.
    pub(crate) fn with_managed_reference_bytes(mut self, managed_reference_bytes: u8) -> Self {
        self.managed_reference_bytes = managed_reference_bytes;

        self
    }

    #[cfg(test)]
    /// Return this image with one explicit size-class table.
    pub(crate) fn with_size_classes(mut self, size_classes: SizeClassTable) -> Self {
        self.size_classes = size_classes;

        self
    }

    /// Return the configured maximum payload size admitted into young space.
    pub(crate) const fn max_young_allocation_bytes(&self) -> usize {
        self.max_young_allocation_bytes
    }

    /// Return the configured remembered-card width.
    pub(crate) const fn card_bytes(&self) -> usize {
        self.card_bytes
    }

    /// Return the configured entry count per metadata table chunk.
    pub(crate) const fn table_chunk_len(&self) -> usize {
        self.table_chunk_len
    }

    /// Return the next managed reference id.
    pub(crate) const fn next_unused_reference_id(&self) -> u64 {
        self.next_unused_reference_id
    }

    /// Return the next managed entry id in large space.
    pub(crate) const fn next_unused_large_entry_id(&self) -> u64 {
        self.next_unused_large_entry_id
    }

    /// Return the allocated managed reference count.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated managed bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured collector state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }
}

impl ManagedSpace {
    /// Fork one managed space over the same shared arena.
    pub(crate) fn fork(&self) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        let page_views = live_page_views(self);
        let mut retained = Vec::new();

        // retain the shared backing before cloning metadata
        for page_view in page_views {
            if let Err(error) = self.arena.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    self.arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        // rebuild the live metadata over retained pages
        let result = (|| {
            let mut space = Self {
                arena: self.arena.clone(),
                page_run_cache: PageRunCache::new(self.arena.pages_per_segment()),
                managed_reference_bytes: self.managed_reference_bytes,
                max_young_allocation_bytes: self.max_young_allocation_bytes,
                card_bytes: self.card_bytes,
                table_chunk_len: self.table_chunk_len,
                edge_table: self.edge_table.clone(),
                young: Self::fork_young_space(self),
                small: Self::fork_small_space(self)?,
                large: Self::fork_large_space(self)?,
                references: self.references.clone(),
                free_reference_ids: self.free_reference_ids.clone(),
                next_unused_reference_id: self.next_unused_reference_id,
                totals: self.totals,
                gc_state: self.gc_state.clone(),
                marks: MarkSet::default(),
                trace_queue: TraceQueue::default(),
                is_collecting: false,
                pins: Default::default(),
                dirty_spans: Vec::new(),
                dirty_large_entries: Vec::new(),
            };

            // rebuild remembered-set state conservatively after fork
            space.rebuild_remembered_set()?;

            Ok(space)
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                self.arena.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Restore one managed space from one frozen managed-space root.
    pub(crate) fn from_image(
        arena: Arc<Arena>,
        image: &ManagedSpaceImage,
    ) -> Result<Self, HeapError> {
        HeapOptions::validate_managed_reference_bytes(image.managed_reference_bytes())?;
        HeapOptions::validate_card_bytes(image.card_bytes())?;
        HeapOptions::validate_table_chunk_len(image.table_chunk_len())?;

        // reject contradictory young-space policy
        if image.young().capacity_bytes() != 0
            && image.max_young_allocation_bytes() > image.young().capacity_bytes()
        {
            return Err(HeapError::ManagedYoungThresholdExceedsCapacity {
                threshold: image.max_young_allocation_bytes(),
                capacity: image.young().capacity_bytes(),
            });
        }

        Self::restore_from_image(arena, image)
    }

    /// Restore one managed space from one checked frozen managed-space root.
    fn restore_from_image(arena: Arc<Arena>, image: &ManagedSpaceImage) -> Result<Self, HeapError> {
        let page_views = image_page_views(image).collect::<Vec<_>>();
        let mut retained = Vec::new();

        // retain the shared backing first
        for page_view in page_views {
            if let Err(error) = arena.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    arena.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        // rebuild the dense metadata tables
        let result = (|| {
            let references = Self::restore_reference_table(image)?;
            let free_reference_ids = Self::free_reference_ids(&references);
            let edge_table = Self::restore_edge_table(image)?;

            // rebuild each live managed storage partition
            let young = Self::restore_young_space(image);
            let small = Self::restore_small_space(image)?;
            let large = Self::restore_large_space(image)?;
            let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
                0
            } else {
                image.max_young_allocation_bytes()
            };

            // rebuild the live root over the shared arena
            let mut space = Self {
                arena: arena.clone(),
                page_run_cache: PageRunCache::new(arena.pages_per_segment()),
                managed_reference_bytes: image.managed_reference_bytes(),
                max_young_allocation_bytes,
                card_bytes: image.card_bytes(),
                table_chunk_len: image.table_chunk_len(),
                edge_table,
                young,
                small,
                large,
                references,
                free_reference_ids,
                next_unused_reference_id: image.next_unused_reference_id(),
                totals: AllocationTotals::new(image.allocated_count(), image.allocated_bytes()),
                gc_state: image.gc_state().clone(),
                marks: MarkSet::default(),
                trace_queue: TraceQueue::default(),
                is_collecting: false,
                pins: Default::default(),
                dirty_spans: Vec::new(),
                dirty_large_entries: Vec::new(),
            };

            // rebuild remembered-set state conservatively after restore
            space.rebuild_remembered_set()?;

            Ok(space)
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                arena.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Return one frozen managed-space root.
    pub(crate) fn image(&self) -> Result<ManagedSpaceImage, HeapError> {
        self.check_branch_boundary()?;

        // capture the live managed storage directly
        let young = self.capture_young_image();
        let spans = self.capture_span_images();
        let entries = self.capture_large_entry_images();
        let references = self.capture_reference_table();
        let edge_maps = self.capture_edge_maps();

        // freeze the current managed root
        Ok(ManagedSpaceImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            entries,
            references,
            edge_maps,
            self.managed_reference_bytes,
            self.young.capacity_bytes,
            self.max_young_allocation_bytes,
            self.card_bytes,
            self.table_chunk_len,
            self.next_unused_reference_id,
            self.large.next_unused_large_entry_id,
            self.totals.allocation_count(),
            self.totals.allocated_bytes(),
            self.gc_state.clone(),
        ))
    }

    /// Check that image and fork boundaries cannot capture transient GC state.
    pub(crate) fn check_branch_boundary(&self) -> Result<(), HeapError> {
        if self.is_collecting {
            return Err(HeapError::CaptureGcActive);
        }

        if self.pins.is_active() {
            return Err(HeapError::CapturePinsActive);
        }

        Ok(())
    }

    /// Rebuild the dense managed reference table from one frozen image.
    fn restore_reference_table(
        image: &ManagedSpaceImage,
    ) -> Result<CowTable<ManagedReferenceEntry>, HeapError> {
        CowTable::from_vec_with_chunk_len(image.references().to_vec(), image.table_chunk_len())
    }

    /// Rebuild the interned edge maps from one frozen image.
    fn restore_edge_table(image: &ManagedSpaceImage) -> Result<EdgeTable, HeapError> {
        EdgeTable::from_edge_maps(image.edge_maps().to_vec())
    }

    /// Return the reusable managed reference ids from one frozen table.
    fn free_reference_ids(references: &CowTable<ManagedReferenceEntry>) -> Vec<u64> {
        references
            .iter()
            .enumerate()
            .filter_map(|(index, record)| record.is_vacant().then_some(index as u64 + 1))
            .collect()
    }

    /// Restore the managed young space from one frozen image.
    fn restore_young_space(image: &ManagedSpaceImage) -> YoungSpace {
        YoungSpace {
            generation: image.young().generation(),
            capacity_bytes: image.young().capacity_bytes(),
            page_bytes: image.young().page_bytes(),
            next_offset: image.young().next_offset(),
            pages: *image.young().pages(),
            entries: image.young().entries().to_vec(),
        }
    }

    /// Fork the managed young space from one live root.
    fn fork_young_space(space: &Self) -> YoungSpace {
        YoungSpace {
            generation: space.young.generation,
            capacity_bytes: space.young.capacity_bytes,
            page_bytes: space.young.page_bytes,
            next_offset: space.young.next_offset,
            pages: space.young.pages,
            entries: space.young.entries.clone(),
        }
    }

    /// Restore the managed small space from one frozen image.
    fn restore_small_space(image: &ManagedSpaceImage) -> Result<super::SmallSpace, HeapError> {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(|span| Self::restore_span(span, image.card_bytes()))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans: CowTable::from_vec_with_chunk_len(spans, image.table_chunk_len())?,
            available_spans: vec![Vec::new(); image.size_classes().classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small)?;

        Ok(small)
    }

    /// Fork the managed small space from one live root.
    fn fork_small_space(space: &Self) -> Result<super::SmallSpace, HeapError> {
        // capture and restore spans so branch-local GC state resets cleanly
        let spans = space
            .small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .map(|span| Self::restore_span(&span, space.card_bytes))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: space.small.size_classes.clone(),
            span_bytes: space.small.span_bytes,
            spans: CowTable::from_vec_with_chunk_len(spans, space.table_chunk_len)?,
            available_spans: vec![Vec::new(); space.small.size_classes.classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small)?;

        Ok(small)
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_available_spans(small: &mut super::SmallSpace) -> Result<(), HeapError> {
        for span_index in 0..small.spans.len() {
            let Some(span) = small.spans.get_mut(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };

            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.next_free_slot = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let Some(class_index) = small.size_classes.class_index_for(span.size_class) else {
                return Err(HeapError::InvalidSizeClass {
                    class_bytes: span.size_class,
                });
            };

            small.available_spans[class_index].push(span_index);
        }

        Ok(())
    }

    /// Restore one managed span from one frozen span root.
    fn restore_span(span: &SmallSpanImage, card_bytes: usize) -> HeapResult<SmallSpan> {
        // rebuild the per-slot tracing table
        let edge_ids = span
            .edge_ids
            .iter()
            .copied()
            .map(|edge_id| EdgeId::from_index(edge_id as usize))
            .collect::<HeapResult<Vec<_>>>()?
            .into_boxed_slice();

        // rebuild the live span around the captured page view
        let dirty_card_bytes =
            span.slot_count
                .checked_mul(span.size_class)
                .ok_or(HeapError::InvariantOverflow {
                    context: "managed span dirty-card bytes",
                })?;

        Ok(SmallSpan {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            occupied: span.occupied.clone(),
            edge_ids,
            layout_ids: span.layout_ids.clone(),
            pages: span.pages,
            dirty_cards: CardSet::with_len(dirty_card_bytes, card_bytes),
            is_dirty_queued: false,
        })
    }

    /// Restore the managed large space from one frozen image.
    fn restore_large_space(image: &ManagedSpaceImage) -> Result<super::LargeSpace, HeapError> {
        // rebuild the captured entry roots first
        let entries = image
            .entries()
            .iter()
            .map(|entry| Self::restore_large_entry(entry, image.card_bytes()))
            .collect::<Vec<_>>();

        // rebuild the reusable entry ids from the frozen table
        let free_large_entry_ids = Self::free_large_entry_ids(image);

        Ok(super::LargeSpace {
            page_bytes: image.page_bytes(),
            entries: CowTable::from_vec_with_chunk_len(entries, image.table_chunk_len())?,
            free_large_entry_ids,
            next_unused_large_entry_id: image.next_unused_large_entry_id(),
        })
    }

    /// Fork the managed large space from one live root.
    fn fork_large_space(space: &Self) -> Result<super::LargeSpace, HeapError> {
        let entries = space
            .large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .map(|entry| Self::restore_large_entry(&entry, space.card_bytes))
            .collect::<Vec<_>>();

        Ok(super::LargeSpace {
            page_bytes: space.large.page_bytes,
            entries: CowTable::from_vec_with_chunk_len(entries, space.table_chunk_len)?,
            free_large_entry_ids: space.large.free_large_entry_ids.clone(),
            next_unused_large_entry_id: space.large.next_unused_large_entry_id,
        })
    }

    /// Restore one managed entry from one frozen entry root.
    fn restore_large_entry(entry: &LargeEntryImage, card_bytes: usize) -> LargeEntry {
        LargeEntry {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages,
            edge_id: entry.edge_id,
            layout_id: entry.layout_id,
            dirty_cards: CardSet::with_len(entry.len, card_bytes),
            is_dirty_queued: false,
        }
    }

    /// Return the reusable managed entry ids from one frozen table.
    fn free_large_entry_ids(image: &ManagedSpaceImage) -> Vec<u64> {
        image
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (!entry.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture the live managed young-space image.
    fn capture_young_image(&self) -> YoungImage {
        YoungImage::new(
            self.young.generation,
            self.young.capacity_bytes,
            self.young.page_bytes,
            self.young.next_offset,
            self.young.pages,
            self.young.entries.clone().into_boxed_slice(),
        )
    }

    /// Capture the dense managed reference table.
    fn capture_reference_table(&self) -> Box<[ManagedReferenceEntry]> {
        self.references.to_boxed_slice()
    }

    /// Capture the interned managed edge maps.
    fn capture_edge_maps(&self) -> Box<[EdgeMap]> {
        self.edge_table.edge_maps().into_boxed_slice()
    }

    /// Capture every live managed span image.
    fn capture_span_images(&self) -> Box<[SmallSpanImage]> {
        self.small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live managed span image.
    fn capture_span_image(span: &SmallSpan) -> SmallSpanImage {
        SmallSpanImage {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            edge_ids: span
                .edge_ids
                .iter()
                .map(|edge_id| edge_id.index() as u32)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            layout_ids: span.layout_ids.clone(),
            pages: span.pages,
        }
    }

    /// Capture every live managed entry image in large space.
    fn capture_large_entry_images(&self) -> Box<[LargeEntryImage]> {
        self.large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live managed entry image in large space.
    fn capture_large_entry_image(entry: &LargeEntry) -> LargeEntryImage {
        LargeEntryImage {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages,
            edge_id: entry.edge_id,
            layout_id: entry.layout_id,
        }
    }
}

/// Return the page views reachable from one frozen managed-space root.
fn image_page_views(image: &ManagedSpaceImage) -> impl DoubleEndedIterator<Item = PageView> + '_ {
    image
        .spans()
        .iter()
        .map(|span| span.pages)
        .chain(image.entries().iter().map(|entry| entry.pages))
        .chain(std::iter::once(*image.young().pages()))
}

/// Return the page views reachable from one live managed space.
pub(crate) fn live_page_views(space: &ManagedSpace) -> Vec<PageView> {
    let mut page_views = Vec::new();

    // collect the span roots first
    page_views.extend(space.small.spans.iter().map(|span| span.pages));

    // collect the large-entry roots next
    page_views.extend(space.large.entries.iter().map(|entry| entry.pages));

    // collect the young-space root last
    page_views.push(space.young.pages);

    page_views
}
