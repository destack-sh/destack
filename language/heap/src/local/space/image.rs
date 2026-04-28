use serde::{Deserialize, Serialize};

use std::sync::Arc;

use super::{
    CardSet, GcState, HeapPageMapEntry, HeapSpace, LargeAllocation, LargeAllocationId,
    LargeAllocationImage, SmallSpan, SmallSpanImage, YoungImage, YoungSpace,
};
use crate::allocator::{AddressSpace, Allocator, PageRun, PageRunCache, SizeClassTable};
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
    /// The reserved virtual byte capacity for heap space.
    space_bytes: usize,
    /// The captured heap allocations in large space.
    allocations: Box<[LargeAllocationImage]>,

    /// The configured young-space byte width.
    young_bytes: usize,
    /// The configured maximum payload size routed to young space.
    max_young_allocation_bytes: usize,
    /// The next heap allocation id to allocate in large space.
    next_unused_large_allocation_id: u64,
    /// The next unused byte offset in heap space.
    next_offset: usize,
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
        space_bytes: usize,
        allocations: Box<[LargeAllocationImage]>,
        young_bytes: usize,
        max_young_allocation_bytes: usize,
        next_unused_large_allocation_id: u64,
        next_offset: usize,
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
            space_bytes,
            allocations,
            young_bytes,
            max_young_allocation_bytes,
            next_unused_large_allocation_id,
            next_offset,
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

    /// Return the reserved virtual byte capacity for heap space.
    pub(crate) const fn space_bytes(&self) -> usize {
        self.space_bytes
    }

    /// Return the captured heap allocations in large space.
    pub(crate) fn allocations(&self) -> &[LargeAllocationImage] {
        &self.allocations
    }

    /// Return this image with one explicit size-class table.
    #[cfg(test)]
    pub(crate) fn with_size_classes(mut self, size_classes: SizeClassTable) -> Self {
        self.size_classes = size_classes;

        self
    }

    /// Return the configured maximum payload size routed to young space.
    pub(crate) const fn max_young_allocation_bytes(&self) -> usize {
        self.max_young_allocation_bytes
    }

    /// Return the next heap allocation id in large space.
    pub(crate) const fn next_unused_large_allocation_id(&self) -> u64 {
        self.next_unused_large_allocation_id
    }

    /// Return the next unused byte offset in heap space.
    pub(crate) const fn next_offset(&self) -> usize {
        self.next_offset
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

    /// Return every live allocator page run captured by this image.
    pub(crate) fn page_runs(&self) -> Vec<PageRun> {
        image_page_runs(self).collect()
    }
}

impl HeapSpace {
    /// Fork one heap space over the same shared allocator.
    pub(crate) fn fork(&mut self) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary()?;

        let mut space = Self::fork_state(self)?;

        // rebuild remembered-set state conservatively after fork
        space
            .rebuild_page_map()
            .and_then(|()| space.rebuild_remembered_set())
            .and_then(|()| space.rebuild_shared_edge_roots())?;

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
        let mut space = Self::restore_state(allocator.clone(), image)?;

        // rebuild remembered-set state conservatively after restore
        space
            .rebuild_page_map()
            .and_then(|()| space.rebuild_remembered_set())
            .and_then(|()| space.rebuild_shared_edge_roots())?;

        Ok(space)
    }

    /// Return one frozen heap-space root.
    pub(crate) fn image(&mut self) -> Result<HeapSpaceImage, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary()?;

        // capture the live heap allocations directly
        let young = self.capture_young_image()?;
        let spans = self.capture_span_images()?;
        let allocations = self.capture_large_allocation_images()?;

        // freeze the current heap root
        Ok(HeapSpaceImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_bytes,
            spans,
            self.large.page_bytes,
            self.mapping.byte_len(),
            allocations,
            self.young.capacity_bytes,
            self.max_young_allocation_bytes,
            self.large.next_unused_large_allocation_id,
            self.next_offset,
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
    fn restore_young_space(
        allocator: &Allocator,
        image: &HeapSpaceImage,
    ) -> HeapResult<YoungSpace> {
        let ranges = image.young().ranges().to_vec();
        let pages = allocator.allocate_pages(image.young().capacity_bytes())?;

        Ok(YoungSpace {
            generation: image.young().generation(),
            capacity_bytes: image.young().capacity_bytes(),
            page_bytes: image.young().page_bytes(),
            next_offset: image.young().next_offset(),
            allocation_alignment_bytes: image.young().allocation_alignment_bytes(),
            pages,
            ranges,
            live: image.young().live().clone(),
            marked: crate::Bitmap::with_capacity(image.young().ranges().len()),
            local_reference_bits: image.young().local_reference_bits().clone(),
            shared_reference_bits: image.young().shared_reference_bits().clone(),
        })
    }

    /// Fork the heap young space from one live root.
    fn fork_young_space(space: &Self) -> HeapResult<YoungSpace> {
        let ranges = space.young.ranges.clone();
        let pages = space.allocator.share_page_run(space.young.pages)?;

        Ok(YoungSpace {
            generation: space.young.generation,
            capacity_bytes: space.young.capacity_bytes,
            page_bytes: space.young.page_bytes,
            next_offset: space.young.next_offset,
            allocation_alignment_bytes: space.young.allocation_alignment_bytes,
            pages,
            ranges,
            live: space.young.live.clone(),
            marked: crate::Bitmap::with_capacity(space.young.ranges.len()),
            local_reference_bits: space.young.local_reference_bits.clone(),
            shared_reference_bits: space.young.shared_reference_bits.clone(),
        })
    }

    /// Fork one heap space over operating-system copy-on-write mapping.
    fn fork_state(space: &mut Self) -> Result<Self, HeapError> {
        let small = Self::fork_small_space(space)?;
        let large = Self::fork_large_space(space)?;
        let young = Self::fork_young_space(space)?;
        let mapping = space.mapping.fork()?;

        Ok(Self {
            allocator: space.allocator.clone(),
            page_run_cache: PageRunCache::new(space.allocator.pages_per_chunk()),
            max_young_allocation_bytes: space.max_young_allocation_bytes,
            young,
            small,
            large,
            page_map: Vec::new(),
            next_offset: space.next_offset,
            mapping,
            usage: space.usage,
            gc: space.gc.clone(),
            trace_queue: TraceQueue::default(),
            major_phase: super::LocalGcPhase::Idle,
            major_trace_queue: TraceQueue::default(),
            major_sweep_references: Vec::new(),
            major_sweep_cursor: 0,
            major_freed_allocations: 0,
            major_freed_bytes: 0,
            is_collecting: false,
            pins: Default::default(),
            dirty_spans: Vec::new(),
            dirty_large_allocations: Vec::new(),
            shared_edge_roots: Vec::new(),
            shared_edge_index: Default::default(),
            is_scanning_shared_edges: false,
            shared_edge_cursor: 0,
            shared_edge_queue: TraceQueue::default(),
            shared_edge_pending: Default::default(),
        })
    }

    /// Restore one heap space into fresh page runs.
    fn restore_state(allocator: Arc<Allocator>, image: &HeapSpaceImage) -> Result<Self, HeapError> {
        let young = Self::restore_young_space(allocator.as_ref(), image)?;
        let small = Self::restore_small_space(allocator.as_ref(), image)?;
        let large = Self::restore_large_space(allocator.as_ref(), image)?;
        let mut mapping = AddressSpace::reserve(image.space_bytes(), image.page_bytes())?;
        restore_image_mapping(&allocator, image, &mut mapping)?;
        let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
            0
        } else {
            image.max_young_allocation_bytes()
        };

        Ok(Self {
            allocator: allocator.clone(),
            page_run_cache: PageRunCache::new(allocator.pages_per_chunk()),
            max_young_allocation_bytes,
            young,
            small,
            large,
            page_map: Vec::new(),
            next_offset: image.next_offset(),
            mapping,
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            gc: image.gc_state().clone(),
            trace_queue: TraceQueue::default(),
            major_phase: super::LocalGcPhase::Idle,
            major_trace_queue: TraceQueue::default(),
            major_sweep_references: Vec::new(),
            major_sweep_cursor: 0,
            major_freed_allocations: 0,
            major_freed_bytes: 0,
            is_collecting: false,
            pins: Default::default(),
            dirty_spans: Vec::new(),
            dirty_large_allocations: Vec::new(),
            shared_edge_roots: Vec::new(),
            shared_edge_index: Default::default(),
            is_scanning_shared_edges: false,
            shared_edge_cursor: 0,
            shared_edge_queue: TraceQueue::default(),
            shared_edge_pending: Default::default(),
        })
    }

    /// Restore the heap small space from one frozen image.
    fn restore_small_space(
        allocator: &Allocator,
        image: &HeapSpaceImage,
    ) -> Result<super::SmallSpace, HeapError> {
        // restore the captured span roots first
        let spans = image
            .spans()
            .iter()
            .map(|span| Self::restore_span(allocator, span))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: image.size_classes().clone(),
            span_bytes: image.small_bytes(),
            spans: CowTable::from_vec(spans),
            partial_spans: vec![
                Vec::new();
                crate::SmallSpanClass::bucket_count(image.size_classes())
            ],
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small)?;

        Ok(small)
    }

    /// Fork the heap small space from one live root.
    fn fork_small_space(space: &Self) -> Result<super::SmallSpace, HeapError> {
        // copy spans from the live mapping
        let spans = space
            .small
            .spans
            .iter()
            .map(|span| Self::fork_span(space, span))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallSpace {
            size_classes: space.small.size_classes.clone(),
            span_bytes: space.small.span_bytes,
            spans: CowTable::from_vec(spans),
            partial_spans: vec![
                Vec::new();
                crate::SmallSpanClass::bucket_count(&space.small.size_classes)
            ],
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small)?;

        Ok(small)
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_partial_spans(small: &mut super::SmallSpace) -> Result<(), HeapError> {
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
            small.partial_spans[bucket_index].push(span_index);
        }

        Ok(())
    }

    /// Restore one heap span from one frozen span root.
    fn restore_span(allocator: &Allocator, span: &SmallSpanImage) -> HeapResult<SmallSpan> {
        // rebuild the live span around fresh pages
        let dirty_card_bytes = span.slot_count * span.class.size_class;
        let byte_len = span.pages.len() * allocator.page_bytes();
        let pages = allocator.allocate_pages(byte_len)?;

        Ok(SmallSpan {
            first_offset: span.first_offset,
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            marked: crate::Bitmap::with_capacity(span.slot_count),
            pages,
            dirty_cards: CardSet::with_len(dirty_card_bytes),
            is_dirty_queued: false,
        })
    }

    /// Fork one heap span into shared metadata pages.
    fn fork_span(space: &Self, span: &SmallSpan) -> HeapResult<SmallSpan> {
        let pages = space.allocator.share_page_run(span.pages)?;

        Ok(SmallSpan {
            first_offset: span.first_offset,
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied_count: span.occupied_count,
            free_cursor: span.free_cursor,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            marked: crate::Bitmap::with_capacity(span.slot_count),
            pages,
            dirty_cards: span.dirty_cards.clone(),
            is_dirty_queued: span.is_dirty_queued,
        })
    }

    /// Restore the heap large space from one frozen image.
    fn restore_large_space(
        allocator: &Allocator,
        image: &HeapSpaceImage,
    ) -> Result<super::LargeSpace, HeapError> {
        // rebuild the captured allocation roots first
        let allocations = image
            .allocations()
            .iter()
            .map(|allocation| Self::restore_large_allocation(allocator, allocation))
            .collect::<HeapResult<Vec<_>>>()?;

        // rebuild the reusable allocation ids from the frozen table
        let free_large_allocation_ids = Self::free_large_allocation_ids(image);

        Ok(super::LargeSpace {
            page_bytes: image.page_bytes(),
            allocations: CowTable::from_vec(allocations),
            free_large_allocation_ids,
            next_unused_large_allocation_id: image.next_unused_large_allocation_id(),
        })
    }

    /// Fork the heap large space from one live root.
    fn fork_large_space(space: &Self) -> Result<super::LargeSpace, HeapError> {
        // copy allocations from the live mapping
        let allocations = space
            .large
            .allocations
            .iter()
            .map(|allocation| Self::fork_large_allocation(space, allocation))
            .collect::<HeapResult<Vec<_>>>()?;

        Ok(super::LargeSpace {
            page_bytes: space.large.page_bytes,
            allocations: CowTable::from_vec(allocations),
            free_large_allocation_ids: space.large.free_large_allocation_ids.clone(),
            next_unused_large_allocation_id: space.large.next_unused_large_allocation_id,
        })
    }

    /// Restore one heap allocation from one frozen allocation root.
    fn restore_large_allocation(
        allocator: &Allocator,
        allocation: &LargeAllocationImage,
    ) -> HeapResult<LargeAllocation> {
        let pages = if allocation.is_live {
            allocator.allocate_pages(allocation.len)?
        } else {
            PageRun::empty()
        };

        Ok(LargeAllocation {
            is_live: allocation.is_live,
            first_offset: allocation.first_offset,
            len: allocation.len,
            pages,
            reference_map: allocation.reference_map.clone(),
            is_marked: false,
            dirty_cards: CardSet::with_len(allocation.len),
            is_dirty_queued: false,
        })
    }

    /// Fork one heap large allocation into shared metadata pages.
    fn fork_large_allocation(
        space: &Self,
        allocation: &LargeAllocation,
    ) -> HeapResult<LargeAllocation> {
        let pages = if allocation.is_live {
            space.allocator.share_page_run(allocation.pages)?
        } else {
            PageRun::empty()
        };

        Ok(LargeAllocation {
            is_live: allocation.is_live,
            first_offset: allocation.first_offset,
            len: allocation.len,
            pages,
            reference_map: allocation.reference_map.clone(),
            is_marked: allocation.is_marked,
            dirty_cards: allocation.dirty_cards.clone(),
            is_dirty_queued: allocation.is_dirty_queued,
        })
    }

    /// Return the reusable heap allocation ids from one frozen table.
    fn free_large_allocation_ids(image: &HeapSpaceImage) -> Vec<u64> {
        image
            .allocations()
            .iter()
            .enumerate()
            .filter_map(|(index, allocation)| (!allocation.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture the live heap young-space image.
    fn capture_young_image(&self) -> HeapResult<YoungImage> {
        let ranges = self.young.ranges.clone();

        // capture the current retained bytes into page roots
        let bytes = self.mapping.bytes(0, self.young.capacity_bytes)?;
        let pages = self.allocator.allocate_bytes(&bytes)?;

        Ok(YoungImage::new(
            self.young.generation,
            self.young.capacity_bytes,
            self.young.page_bytes,
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
            pages,
            ranges.into_boxed_slice(),
            self.young.live.clone(),
            self.young.local_reference_bits.clone(),
            self.young.shared_reference_bits.clone(),
        ))
    }

    /// Capture every live heap span image.
    fn capture_span_images(&self) -> HeapResult<Box<[SmallSpanImage]>> {
        self.small
            .spans
            .iter()
            .map(|span| self.capture_span_image(span))
            .collect::<HeapResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Capture one live heap span image.
    fn capture_span_image(&self, span: &SmallSpan) -> HeapResult<SmallSpanImage> {
        let byte_len = span.pages.len() * self.allocator.page_bytes();

        // capture the current retained bytes into page roots
        let bytes = self.mapping.bytes(span.first_offset, byte_len)?;
        let pages = self.allocator.allocate_bytes(&bytes)?;

        Ok(SmallSpanImage {
            first_offset: span.first_offset,
            class: span.class.clone(),
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            pages,
        })
    }

    /// Capture every live heap allocation image in large space.
    fn capture_large_allocation_images(&self) -> HeapResult<Box<[LargeAllocationImage]>> {
        self.large
            .allocations
            .iter()
            .map(|allocation| self.capture_large_allocation_image(allocation))
            .collect::<HeapResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Capture one live heap allocation image in large space.
    fn capture_large_allocation_image(
        &self,
        allocation: &LargeAllocation,
    ) -> HeapResult<LargeAllocationImage> {
        // capture the current retained bytes into page roots
        let pages = if allocation.is_live {
            let bytes = self
                .mapping
                .bytes(allocation.first_offset, allocation.len)?;

            self.allocator.allocate_bytes(&bytes)?
        } else {
            PageRun::empty()
        };

        Ok(LargeAllocationImage {
            is_live: allocation.is_live,
            first_offset: allocation.first_offset,
            len: allocation.len,
            pages,
            reference_map: allocation.reference_map.clone(),
        })
    }
}

/// Return the page runs reachable from one frozen heap-space root.
fn image_page_runs(image: &HeapSpaceImage) -> impl DoubleEndedIterator<Item = PageRun> + '_ {
    image
        .spans()
        .iter()
        .map(|span| span.pages)
        .chain(
            image
                .allocations()
                .iter()
                .map(|allocation| allocation.pages),
        )
        .chain(std::iter::once(*image.young().pages()))
}

/// Restore one heap mapping from one image root.
fn restore_image_mapping(
    allocator: &Allocator,
    image: &HeapSpaceImage,
    mapping: &mut AddressSpace,
) -> HeapResult<()> {
    let young_bytes = image.young().capacity_bytes();

    // restore the young mapped range first
    if young_bytes != 0 {
        let bytes = allocator.bytes_to_vec_from(image.young().pages(), 0, young_bytes)?;

        mapping.write(0, &bytes)?;
    }

    // restore each captured small span range
    for span in image.spans() {
        let byte_len = span.pages.len() * allocator.page_bytes();
        if byte_len == 0 {
            continue;
        }

        let bytes = allocator.bytes_to_vec_from(&span.pages, 0, byte_len)?;

        mapping.write(span.first_offset, &bytes)?;
    }

    // restore each captured large allocation range
    for allocation in image.allocations() {
        if !allocation.is_live || allocation.len == 0 {
            continue;
        }

        let bytes = allocator.bytes_to_vec_from(&allocation.pages, 0, allocation.len)?;

        mapping.write(allocation.first_offset, &bytes)?;
    }

    Ok(())
}

impl HeapSpace {
    /// Rebuild the page-map table from live place.
    fn rebuild_page_map(&mut self) -> HeapResult<()> {
        self.page_map.clear();

        let young_pages = self.young.pages;

        self.map_page_run(0, &young_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::MissingSpan { span_index });
            };
            let pages = span.pages;

            self.map_page_run(span.first_offset, &pages, |logical_page_index| {
                HeapPageMapEntry::Small {
                    span_index,
                    logical_page_index,
                }
            });
        }

        for allocation_index in 0..self.large.allocations.len() {
            let allocation_id = LargeAllocationId::new(allocation_index as u64 + 1);
            let Some(allocation) = self.large_allocation(allocation_id) else {
                continue;
            };
            let pages = allocation.pages;

            self.map_page_run(allocation.first_offset, &pages, |logical_page_index| {
                HeapPageMapEntry::Large {
                    allocation_id,
                    logical_page_index,
                }
            });
        }

        Ok(())
    }
}
