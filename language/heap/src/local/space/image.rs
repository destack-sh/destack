use serde::{Deserialize, Serialize};

use std::sync::Arc;

use destack_mir::LayoutTable;

use super::{
    CardSet, GcState, HeapPageOwner, HeapSpace, LargeEntry, LargeEntryId, LargeEntryImage,
    SmallSpan, SmallSpanImage, YoungImage, YoungSpace,
};
use crate::allocator::{Allocator, PageRunCache, PageView, SizeClassTable};
use crate::{AllocationUsage, CowTable, HeapError, HeapResult, TraceQueue};

/// One frozen heap-space root.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// The canonical managed layouts visible to this heap.
    layouts: LayoutTable,

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
    gc_state: GcState,
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
        layouts: LayoutTable,
        young_bytes: usize,
        max_young_allocation_bytes: usize,
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
            layouts,
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

    /// Return the canonical managed layouts visible to this heap.
    pub(crate) fn layouts(&self) -> &LayoutTable {
        &self.layouts
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
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }
}

impl HeapSpace {
    /// Fork one heap space over the same shared allocator.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary();

        let page_views = live_page_views(self);
        let retained_page_views = self.allocator.retain_page_views(page_views)?;

        let mut space = match Self::fork_with_retained_pages(self) {
            Ok(space) => space,
            Err(error) => {
                self.allocator.release_page_views(&retained_page_views)?;

                return Err(error);
            }
        };

        // rebuild remembered-set state conservatively after fork
        if let Err(error) = space
            .rebuild_page_owners()
            .and_then(|()| space.rebuild_remembered_set())
            .and_then(|()| space.rebuild_shared_edge_roots())
        {
            self.allocator.release_page_views(&retained_page_views)?;

            return Err(error);
        }

        Ok(space)
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
        let retained_page_views = allocator.retain_page_views(image_page_views(image))?;

        let mut space = match Self::restore_with_retained_pages(allocator.clone(), image) {
            Ok(space) => space,
            Err(error) => {
                allocator.release_page_views(&retained_page_views)?;

                return Err(error);
            }
        };

        // rebuild remembered-set state conservatively after restore
        if let Err(error) = space
            .rebuild_page_owners()
            .and_then(|()| space.rebuild_remembered_set())
            .and_then(|()| space.rebuild_shared_edge_roots())
        {
            allocator.release_page_views(&retained_page_views)?;

            return Err(error);
        }

        Ok(space)
    }

    /// Return one frozen heap-space root.
    pub(crate) fn image(&mut self) -> Result<HeapSpaceImage, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary();

        // capture the live heap storage directly
        let young = self.capture_young_image();
        let spans = self.capture_span_images();
        let entries = self.capture_large_entry_images();
        let layouts = self.capture_layouts();

        // freeze the current heap root
        Ok(HeapSpaceImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            entries,
            layouts,
            self.young.capacity_bytes,
            self.max_young_allocation_bytes,
            self.large.next_unused_large_entry_id,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
            self.gc.clone(),
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

    /// Fork one heap space after retaining all live page views.
    fn fork_with_retained_pages(space: &Self) -> Result<Self, HeapError> {
        let small = Self::fork_small_space(space)?;
        let large = Self::fork_large_space(space)?;

        Ok(Self {
            allocator: space.allocator.clone(),
            page_run_cache: PageRunCache::new(space.allocator.pages_per_arena()),
            max_young_allocation_bytes: space.max_young_allocation_bytes,
            layouts: space.layouts.clone(),
            young: Self::fork_young_space(space),
            small,
            large,
            page_owners: Vec::new(),
            usage: space.usage,
            gc: space.gc.clone(),
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
        })
    }

    /// Restore one heap space after retaining every image page view.
    fn restore_with_retained_pages(
        allocator: Arc<Allocator>,
        image: &HeapSpaceImage,
    ) -> Result<Self, HeapError> {
        let small = Self::restore_small_space(image)?;
        let large = Self::restore_large_space(image)?;
        let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
            0
        } else {
            image.max_young_allocation_bytes()
        };

        Ok(Self {
            allocator: allocator.clone(),
            layouts: Arc::new(image.layouts().clone()),
            page_run_cache: PageRunCache::new(allocator.pages_per_arena()),
            max_young_allocation_bytes,
            young: Self::restore_young_space(image),
            small,
            large,
            page_owners: Vec::new(),
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            gc: image.gc_state().clone(),
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
        })
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
            available_spans: vec![
                Vec::new();
                crate::SmallSpanClass::bucket_count(image.size_classes())
            ],
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
            available_spans: vec![
                Vec::new();
                crate::SmallSpanClass::bucket_count(&space.small.size_classes)
            ],
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
            span.free_cursor = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            let bucket_index = span.class.bucket_index(&small.size_classes)?;
            small.available_spans[bucket_index].push(span_index);
        }

        Ok(())
    }

    /// Restore one heap span from one frozen span root.
    fn restore_span(span: &SmallSpanImage) -> HeapResult<SmallSpan> {
        // rebuild the live span around the captured page view
        let dirty_card_bytes = span.slot_count.checked_mul(span.class.size_class).ok_or(
            HeapError::InvariantOverflow {
                context: "heap span dirty-card bytes",
            },
        )?;

        Ok(SmallSpan {
            class: span.class.clone(),
            slot_count: span.slot_count,
            byte_lens: span.byte_lens.clone(),
            occupied_count: 0,
            free_cursor: 0,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            marked: crate::Bitmap::with_capacity(span.slot_count),
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
            reference_map: entry.reference_map.clone(),
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

    /// Capture the canonical managed layouts visible to this heap.
    fn capture_layouts(&self) -> LayoutTable {
        (*self.layouts).clone()
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
            class: span.class.clone(),
            slot_count: span.slot_count,
            byte_lens: span.byte_lens.clone(),
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
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
            reference_map: entry.reference_map.clone(),
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
