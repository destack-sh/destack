use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    CardSet, GcSummary, HeapPageOwner, HeapSpace, LargeEntry, LargeEntryId, LargeEntryImage,
    SmallSpan, SmallSpanImage, YoungImage, YoungSpace,
};
use crate::allocator::{Allocator, PageRunCache, PageView, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapResult, Shape, ShapeTable, TraceQueue};

/// One frozen heap-space root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HeapSpaceImage {
    /// The captured branchable young-space root.
    young: YoungImage,

    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured heap spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured local page width.
    page_bytes: usize,
    /// The captured heap entries in large space.
    entries: Box<[LargeEntryImage]>,

    /// The captured entry shapes.
    shapes: Box<[Shape]>,

    /// The configured young-space byte width.
    young_bytes: usize,
    /// The configured maximum payload size admitted into young space.
    max_young_allocation_bytes: usize,
    /// The next heap entry id to allocate in large space.
    next_unused_large_entry_id: u64,
    /// The number of allocated heap references.
    allocated_count: usize,
    /// The number of allocated heap bytes.
    allocated_bytes: u64,

    /// The captured GC state.
    gc_state: GcSummary,
}

#[allow(clippy::too_many_arguments)]
impl HeapSpaceImage {
    /// Create one frozen heap-space root.
    pub(crate) fn new(
        young: YoungImage,
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_bytes: usize,
        entries: Box<[LargeEntryImage]>,
        shapes: Box<[Shape]>,
        young_bytes: usize,
        max_young_allocation_bytes: usize,
        next_unused_large_entry_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcSummary,
    ) -> Self {
        Self {
            young,
            size_classes,
            small_bytes,
            spans,
            page_bytes,
            entries,
            shapes,
            young_bytes,
            max_young_allocation_bytes,
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

    /// Return the captured heap spans.
    pub(crate) fn spans(&self) -> &[SmallSpanImage] {
        &self.spans
    }

    /// Return the configured local page width.
    pub(crate) const fn page_bytes(&self) -> usize {
        self.page_bytes
    }

    /// Return the captured heap entries in large space.
    pub(crate) fn entries(&self) -> &[LargeEntryImage] {
        &self.entries
    }

    /// Return the captured entry shapes.
    pub(crate) fn shapes(&self) -> &[Shape] {
        &self.shapes
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

    /// Return the next heap entry id in large space.
    pub(crate) const fn next_unused_large_entry_id(&self) -> u64 {
        self.next_unused_large_entry_id
    }

    /// Return the allocated heap reference count.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated heap bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the captured collector state.
    pub(crate) fn gc_state(&self) -> &GcSummary {
        &self.gc_state
    }
}

impl HeapSpace {
    /// Fork one heap space over the same shared allocator.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary();
        let page_views = live_page_views(self);
        let mut retained = Vec::new();

        // retain the shared backing before cloning metadata
        for page_view in page_views {
            if let Err(error) = self.allocator.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    self.allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        // rebuild the live metadata over retained pages
        let result = (|| {
            let mut space = Self {
                allocator: self.allocator.clone(),
                page_run_cache: PageRunCache::new(self.allocator.pages_per_segment()),
                max_young_allocation_bytes: self.max_young_allocation_bytes,
                shape_table: self.shape_table.clone(),
                young: Self::fork_young_space(self),
                small: Self::fork_small_space(self)?,
                large: Self::fork_large_space(self)?,
                page_owners: Vec::new(),
                usage: self.usage,
                gc_state: self.gc_state.clone(),
                trace_queue: TraceQueue::default(),
                is_collecting: false,
                pins: Default::default(),
                dirty_spans: Vec::new(),
                dirty_large_entries: Vec::new(),
                shared_edge_roots: Vec::new(),
                shared_edge_index: Default::default(),
                is_scanning_shared_edges: false,
                shared_edge_cursor: 0,
                shared_edge_queue: TraceQueue::default(),
                shared_edge_pending: Default::default(),
            };

            space.rebuild_page_owners()?;

            // rebuild remembered-set state conservatively after fork
            space.rebuild_remembered_set()?;
            space.rebuild_shared_edge_roots()?;

            Ok(space)
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                self.allocator.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Restore one heap space from one frozen heap-space root.
    pub(crate) fn from_image(
        allocator: Arc<Allocator>,
        image: &HeapSpaceImage,
    ) -> Result<Self, HeapError> {
        // reject contradictory young-space policy
        if image.young().capacity_bytes() != 0
            && image.max_young_allocation_bytes() > image.young().capacity_bytes()
        {
            return Err(HeapError::HeapYoungThresholdExceedsCapacity {
                threshold: image.max_young_allocation_bytes(),
                capacity: image.young().capacity_bytes(),
            });
        }

        Self::restore_from_image(allocator, image)
    }

    /// Restore one heap space from one checked frozen heap-space root.
    fn restore_from_image(
        allocator: Arc<Allocator>,
        image: &HeapSpaceImage,
    ) -> Result<Self, HeapError> {
        let page_views = image_page_views(image).collect::<Vec<_>>();
        let mut retained = Vec::new();

        // retain the shared backing first
        for page_view in page_views {
            if let Err(error) = allocator.retain_page_view(&page_view) {
                for page_view in retained.into_iter().rev() {
                    allocator.release_page_view(&page_view)?;
                }

                return Err(error);
            }

            retained.push(page_view);
        }

        // rebuild the dense metadata tables
        let result = (|| {
            let shape_table = Self::restore_shape_table(image)?;

            // rebuild each live heap storage partition
            let young = Self::restore_young_space(image);
            let small = Self::restore_small_space(image)?;
            let large = Self::restore_large_space(image)?;
            let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
                0
            } else {
                image.max_young_allocation_bytes()
            };

            // rebuild the live root over the shared allocator
            let mut space = Self {
                allocator: allocator.clone(),
                page_run_cache: PageRunCache::new(allocator.pages_per_segment()),
                max_young_allocation_bytes,
                shape_table,
                young,
                small,
                large,
                page_owners: Vec::new(),
                usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
                gc_state: image.gc_state().clone(),
                trace_queue: TraceQueue::default(),
                is_collecting: false,
                pins: Default::default(),
                dirty_spans: Vec::new(),
                dirty_large_entries: Vec::new(),
                shared_edge_roots: Vec::new(),
                shared_edge_index: Default::default(),
                is_scanning_shared_edges: false,
                shared_edge_cursor: 0,
                shared_edge_queue: TraceQueue::default(),
                shared_edge_pending: Default::default(),
            };

            space.rebuild_page_owners()?;

            // rebuild remembered-set state conservatively after restore
            space.rebuild_remembered_set()?;
            space.rebuild_shared_edge_roots()?;

            Ok(space)
        })();

        if let Err(error) = result {
            for page_view in retained.into_iter().rev() {
                allocator.release_page_view(&page_view)?;
            }

            return Err(error);
        }

        result
    }

    /// Return one frozen heap-space root.
    pub(crate) fn image(&mut self) -> Result<HeapSpaceImage, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary();

        // capture the live heap storage directly
        let young = self.capture_young_image();
        let spans = self.capture_span_images();
        let entries = self.capture_large_entry_images();
        let shapes = self.capture_shapes();

        // freeze the current heap root
        Ok(HeapSpaceImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            entries,
            shapes,
            self.young.capacity_bytes,
            self.max_young_allocation_bytes,
            self.large.next_unused_large_entry_id,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
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

    /// Rebuild the interned entry shapes from one frozen image.
    fn restore_shape_table(image: &HeapSpaceImage) -> Result<ShapeTable, HeapError> {
        ShapeTable::from_shapes(image.shapes().to_vec())
    }

    /// Restore the heap young space from one frozen image.
    fn restore_young_space(image: &HeapSpaceImage) -> YoungSpace {
        let mut entries = image.young().entries().to_vec();

        for entry in &mut entries {
            entry.is_marked = false;
        }

        YoungSpace {
            generation: image.young().generation(),
            capacity_bytes: image.young().capacity_bytes(),
            page_bytes: image.young().page_bytes(),
            next_offset: image.young().next_offset(),
            pages: image.young().pages().clone(),
            entries,
        }
    }

    /// Fork the heap young space from one live root.
    fn fork_young_space(space: &Self) -> YoungSpace {
        let mut entries = space.young.entries.clone();

        for entry in &mut entries {
            entry.is_marked = false;
        }

        YoungSpace {
            generation: space.young.generation,
            capacity_bytes: space.young.capacity_bytes,
            page_bytes: space.young.page_bytes,
            next_offset: space.young.next_offset,
            pages: space.young.pages.clone(),
            entries,
        }
    }

    /// Restore the heap small space from one frozen image.
    fn restore_small_space(image: &HeapSpaceImage) -> Result<super::SmallSpace, HeapError> {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(Self::restore_span)
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans: CowTable::from_vec(spans)?,
            available_spans: vec![Vec::new(); image.size_classes().classes.len()],
        };

        // rebuild the derived span occupancy state
        Self::restore_available_spans(&mut small)?;

        Ok(small)
    }

    /// Fork the heap small space from one live root.
    fn fork_small_space(space: &Self) -> Result<super::SmallSpace, HeapError> {
        // capture and restore spans so branch-local GC state resets cleanly
        let spans = space
            .small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .map(|span| Self::restore_span(&span))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: space.small.size_classes.clone(),
            span_bytes: space.small.span_bytes,
            spans: CowTable::from_vec(spans)?,
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

    /// Restore one heap span from one frozen span root.
    fn restore_span(span: &SmallSpanImage) -> HeapResult<SmallSpan> {
        // rebuild the per-slot tracing table
        let shape_ids = span.shape_ids.clone();

        // rebuild the live span around the captured page view
        let dirty_card_bytes =
            span.slot_count
                .checked_mul(span.size_class)
                .ok_or(HeapError::InvariantOverflow {
                    context: "heap span dirty-card bytes",
                })?;

        Ok(SmallSpan {
            size_class: span.size_class,
            slot_count: span.slot_count,
            occupied_count: 0,
            next_free_slot: 0,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            marked: crate::Bitmap::with_capacity(span.slot_count),
            shape_ids,
            pages: span.pages.clone(),
            dirty_cards: CardSet::with_len(dirty_card_bytes),
            is_dirty_queued: false,
        })
    }

    /// Restore the heap large space from one frozen image.
    fn restore_large_space(image: &HeapSpaceImage) -> Result<super::LargeSpace, HeapError> {
        // rebuild the captured entry roots first
        let entries = image
            .entries()
            .iter()
            .map(Self::restore_large_entry)
            .collect::<Vec<_>>();

        // rebuild the reusable entry ids from the frozen table
        let free_large_entry_ids = Self::free_large_entry_ids(image);

        Ok(super::LargeSpace {
            page_bytes: image.page_bytes(),
            entries: CowTable::from_vec(entries)?,
            free_large_entry_ids,
            next_unused_large_entry_id: image.next_unused_large_entry_id(),
        })
    }

    /// Fork the heap large space from one live root.
    fn fork_large_space(space: &Self) -> Result<super::LargeSpace, HeapError> {
        let entries = space
            .large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .map(|entry| Self::restore_large_entry(&entry))
            .collect::<Vec<_>>();

        Ok(super::LargeSpace {
            page_bytes: space.large.page_bytes,
            entries: CowTable::from_vec(entries)?,
            free_large_entry_ids: space.large.free_large_entry_ids.clone(),
            next_unused_large_entry_id: space.large.next_unused_large_entry_id,
        })
    }

    /// Restore one heap entry from one frozen entry root.
    fn restore_large_entry(entry: &LargeEntryImage) -> LargeEntry {
        LargeEntry {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages.clone(),
            shape_id: entry.shape_id,
            is_marked: false,
            dirty_cards: CardSet::with_len(entry.len),
            is_dirty_queued: false,
        }
    }

    /// Return the reusable heap entry ids from one frozen table.
    fn free_large_entry_ids(image: &HeapSpaceImage) -> Vec<u64> {
        image
            .entries()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| (!entry.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture the live heap young-space image.
    fn capture_young_image(&self) -> YoungImage {
        let mut entries = self.young.entries.clone();

        for entry in &mut entries {
            entry.is_marked = false;
        }

        YoungImage::new(
            self.young.generation,
            self.young.capacity_bytes,
            self.young.page_bytes,
            self.young.next_offset,
            self.young.pages.clone(),
            entries.into_boxed_slice(),
        )
    }

    /// Capture the interned heap entry shapes.
    fn capture_shapes(&self) -> Box<[Shape]> {
        self.shape_table.shapes().to_vec().into_boxed_slice()
    }

    /// Capture every live heap span image.
    fn capture_span_images(&self) -> Box<[SmallSpanImage]> {
        self.small
            .spans
            .iter()
            .map(Self::capture_span_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live heap span image.
    fn capture_span_image(span: &SmallSpan) -> SmallSpanImage {
        SmallSpanImage {
            size_class: span.size_class,
            slot_count: span.slot_count,
            lengths: span.lengths.clone(),
            occupied: span.occupied.clone(),
            shape_ids: span
                .shape_ids
                .iter()
                .copied()
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            pages: span.pages.clone(),
        }
    }

    /// Capture every live heap entry image in large space.
    fn capture_large_entry_images(&self) -> Box<[LargeEntryImage]> {
        self.large
            .entries
            .iter()
            .map(Self::capture_large_entry_image)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live heap entry image in large space.
    fn capture_large_entry_image(entry: &LargeEntry) -> LargeEntryImage {
        LargeEntryImage {
            is_live: entry.is_live,
            len: entry.len,
            pages: entry.pages.clone(),
            shape_id: entry.shape_id,
        }
    }
}

/// Return the page views reachable from one frozen heap-space root.
fn image_page_views(image: &HeapSpaceImage) -> impl DoubleEndedIterator<Item = PageView> + '_ {
    image
        .spans()
        .iter()
        .map(|span| span.pages.clone())
        .chain(image.entries().iter().map(|entry| entry.pages.clone()))
        .chain(std::iter::once(image.young().pages().clone()))
}

/// Return the page views reachable from one live heap space.
pub(crate) fn live_page_views(space: &HeapSpace) -> Vec<PageView> {
    let mut page_views = Vec::new();

    // collect the span roots first
    page_views.extend(space.small.spans.iter().map(|span| span.pages.clone()));

    // collect the large-entry roots next
    page_views.extend(space.large.entries.iter().map(|entry| entry.pages.clone()));

    // collect the young-space root last
    page_views.push(space.young.pages.clone());

    page_views
}

impl HeapSpace {
    /// Rebuild the page-owner table from live storage.
    fn rebuild_page_owners(&mut self) -> HeapResult<()> {
        self.page_owners.clear();

        let young_pages = self.young.pages.clone();

        self.map_page_view(&young_pages, |logical_page_index| HeapPageOwner::Young {
            logical_page_index,
        })?;

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let pages = span.pages.clone();

            self.map_page_view(&pages, |logical_page_index| HeapPageOwner::Small {
                span_index,
                logical_page_index,
            })?;
        }

        for entry_index in 0..self.large.entries.len() {
            let entry_id = LargeEntryId::new(entry_index as u64 + 1);
            let Some(entry) = self.large_entry(entry_id) else {
                continue;
            };
            let pages = entry.pages.clone();

            self.map_page_view(&pages, |logical_page_index| HeapPageOwner::Large {
                entry_id,
                logical_page_index,
            })?;
        }

        Ok(())
    }
}
