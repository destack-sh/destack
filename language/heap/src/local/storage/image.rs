use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::AddressSpace;
use destack_mir::TraceTable;
use serde::{Deserialize, Serialize};

use super::{
    CardSet, GcState, HeapPageMapEntry, HeapStorage, LargeBlock, LargeBlockId, LargeBlockImage,
    SmallSpan, SmallSpanImage, YoungImage, YoungSpace,
};
use crate::allocator::{Allocator, PageSpan, PageSpanCache, SizeClassTable};
use crate::{
    AllocationUsage, Bitmap, CowTable, HeapAllocationError, HeapCaptureBlocker,
    HeapConfigurationError, HeapError, HeapResult, allocation_class,
};

/// One frozen heap storage image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HeapStorageImage {
    /// The captured branchable young space image.
    young: YoungImage,

    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured heap spans.
    spans: Box<[SmallSpanImage]>,
    /// The configured local page width.
    page_size_bytes: usize,
    /// The reserved virtual byte capacity for heap storage.
    address_space_size_bytes: usize,
    /// The captured heap blocks in large space.
    blocks: Box<[LargeBlockImage]>,

    /// The configured young space byte width.
    young_size_bytes: usize,
    /// The configured maximum payload size routed to young space.
    max_young_allocation_bytes: usize,
    /// The next heap block id to allocate in large space.
    next_unused_large_block_id: u64,
    /// The next unused byte offset in heap storage.
    next_offset: usize,
    /// The number of allocated heap references.
    allocated_count: usize,
    /// The number of allocated heap bytes.
    allocated_bytes: u64,

    /// The captured GC state.
    gc_state: GcState,
}

#[allow(clippy::too_many_arguments)]
impl HeapStorageImage {
    /// Create one frozen heap storage image.
    pub(crate) fn new(
        young: YoungImage,
        size_classes: SizeClassTable,
        small_bytes: usize,
        spans: Box<[SmallSpanImage]>,
        page_size_bytes: usize,
        address_space_size_bytes: usize,
        blocks: Box<[LargeBlockImage]>,
        young_size_bytes: usize,
        max_young_allocation_bytes: usize,
        next_unused_large_block_id: u64,
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
            page_size_bytes,
            address_space_size_bytes,
            blocks,
            young_size_bytes,
            max_young_allocation_bytes,
            next_unused_large_block_id,
            next_offset,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the branchable young space image.
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
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the reserved virtual byte capacity for heap storage.
    pub(crate) const fn address_space_size_bytes(&self) -> usize {
        self.address_space_size_bytes
    }

    /// Return the captured heap blocks in large space.
    pub(crate) fn blocks(&self) -> &[LargeBlockImage] {
        &self.blocks
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

    /// Return the next heap block id in large space.
    pub(crate) const fn next_unused_large_block_id(&self) -> u64 {
        self.next_unused_large_block_id
    }

    /// Return the next unused byte offset in heap storage.
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

    /// Return the number of pages needed to restore this image.
    pub(crate) fn page_count(&self) -> usize {
        let young_pages = self.young().bytes().len().div_ceil(self.page_size_bytes);
        let span_pages = self
            .spans()
            .iter()
            .map(|span| span.bytes.len().div_ceil(self.page_size_bytes))
            .sum::<usize>();
        let block_pages = self
            .blocks()
            .iter()
            .filter(|block| block.is_live)
            .map(|block| block.bytes.len().div_ceil(self.page_size_bytes))
            .sum::<usize>();

        young_pages + span_pages + block_pages
    }

    /// Return the captured collector state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }
}

impl HeapStorage {
    /// Fork one heap storage over the same shared allocator.
    ///
    /// Call this only from a safepoint where the heap storage cannot mutate.
    pub(crate) fn fork(&mut self, trace_table: &TraceTable) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary()?;

        let mut space = Self::fork_state(self)?;

        // rebuild remembered-set state conservatively after fork
        space
            .rebuild_page_map()
            .and_then(|()| space.rebuild_remembered_set(trace_table))
            .and_then(|()| space.rebuild_shared_edge_roots(trace_table))?;

        Ok(space)
    }

    /// Restore one heap storage from one frozen heap storage image.
    pub(crate) fn from_image(
        allocator: Arc<Allocator>,
        image: &HeapStorageImage,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        // reject contradictory young space policy
        if image.young().capacity_bytes() != 0
            && image.max_young_allocation_bytes() > image.young().capacity_bytes()
        {
            return Err(HeapError::configuration(
                HeapConfigurationError::YoungThresholdExceedsCapacity {
                    threshold: image.max_young_allocation_bytes(),
                    capacity: image.young().capacity_bytes(),
                },
            ));
        }

        Self::restore_from_image(allocator, image, trace_table)
    }

    /// Restore one heap storage from one checked frozen heap storage image.
    fn restore_from_image(
        allocator: Arc<Allocator>,
        image: &HeapStorageImage,
        trace_table: &TraceTable,
    ) -> Result<Self, HeapError> {
        let mut space = Self::restore_state(allocator.clone(), image)?;

        // rebuild remembered-set state conservatively after restore
        space
            .rebuild_page_map()
            .and_then(|()| space.rebuild_remembered_set(trace_table))
            .and_then(|()| space.rebuild_shared_edge_roots(trace_table))?;

        Ok(space)
    }

    /// Return one frozen heap storage image.
    pub(crate) fn image(&mut self) -> Result<HeapStorageImage, HeapError> {
        self.check_branch_boundary()?;
        self.flush_branch_boundary()?;

        // capture the live heap blocks directly
        let young = self.capture_young_image()?;
        let spans = self.capture_span_images()?;
        let blocks = self.capture_large_block_images()?;

        // freeze the current heap image
        Ok(HeapStorageImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_size_bytes,
            spans,
            self.allocator.page_size_bytes(),
            self.mapping.byte_len(),
            blocks,
            self.young.capacity_bytes,
            self.max_young_allocation_bytes,
            self.large.next_unused_large_block_id,
            self.next_offset,
            self.usage.allocation_count(),
            self.usage.allocated_bytes(),
            self.gc.clone(),
        ))
    }

    /// Check that image and fork boundaries cannot capture transient GC state.
    pub(crate) fn check_branch_boundary(&self) -> Result<(), HeapError> {
        if self.collector.is_collecting() {
            return Err(HeapError::capture_blocked(HeapCaptureBlocker::GcActive));
        }

        if self.collector.pins.is_active() {
            return Err(HeapError::capture_blocked(HeapCaptureBlocker::PinsActive));
        }

        Ok(())
    }

    /// Restore the heap young space from one frozen image.
    fn restore_young_space(
        allocator: &Allocator,
        image: &HeapStorageImage,
    ) -> HeapResult<YoungSpace> {
        let pages = allocator.allocate_pages(image.young().bytes().len())?;
        let ranges = image.young().ranges().to_vec();
        let live = image.young().live().clone();
        let spans = image.young().spans().to_vec();
        let span_bits = image.young().span_bits().to_vec();
        let mut span_cache = Vec::new();
        let page_count = image
            .young()
            .capacity_bytes()
            .div_ceil(image.young().page_size_bytes());
        let mut page_spans = vec![None; page_count];

        for (span_index, span) in spans.iter().enumerate() {
            span.class().validate(
                image.size_classes(),
                image.young().page_size_bytes(),
                image.small_bytes(),
            )?;
            if span.byte_len() == 0 || span.byte_len() > span.class().size_class {
                return Err(HeapError::invalid_allocation(
                    HeapAllocationError::ByteLengthMismatch {
                        expected: span.class().size_class,
                        actual: span.byte_len(),
                    },
                ));
            }

            let class = allocation_class(
                span.byte_len(),
                image.young().allocation_alignment_bytes(),
                span.class().trace_id,
                span.class().is_noscan,
                image.size_classes(),
                image.young().page_size_bytes(),
                image.small_bytes(),
            );
            let Some(small) = class.small() else {
                return Err(HeapError::invalid_allocation(
                    HeapAllocationError::ByteLengthMismatch {
                        expected: span.class().size_class,
                        actual: span.byte_len(),
                    },
                ));
            };
            let cache_index = small.cache_index();
            if span_cache.len() <= cache_index {
                span_cache.resize(cache_index + 1, None);
            }
            if span.next_offset < span.end_offset {
                span_cache[cache_index] = Some(span_index);
            }

            let page_start = span.first_offset / image.young().page_size_bytes();
            let page_count = span.span_size_bytes() / image.young().page_size_bytes();
            for page_span in page_spans.iter_mut().skip(page_start).take(page_count) {
                *page_span = Some(span_index);
            }
        }
        Ok(YoungSpace {
            capacity_bytes: image.young().capacity_bytes(),
            page_size_bytes: image.young().page_size_bytes(),
            next_offset: image.young().next_offset(),
            mapped_until: image.young().capacity_bytes(),
            allocation_alignment_bytes: image.young().allocation_alignment_bytes(),
            pages,
            ranges,
            live,
            marked: Bitmap::with_capacity(image.young().ranges().len()),
            local_reference_bits: image.young().local_reference_bits().clone(),
            shared_reference_bits: image.young().shared_reference_bits().clone(),
            spans,
            span_bits,
            span_cache,
            cursor: None,
            page_spans,
            pending_range_usage: AllocationUsage::default(),
        })
    }

    /// Fork the heap young space from one live space.
    fn fork_young_space(space: &Self) -> HeapResult<YoungSpace> {
        let pages = space.allocator.share_page_span(space.young.pages)?;

        for span in &space.young.spans {
            span.class().validate(
                &space.small.size_classes,
                space.young.page_size_bytes,
                space.small.span_size_bytes,
            )?;
        }

        Ok(YoungSpace {
            capacity_bytes: space.young.capacity_bytes,
            page_size_bytes: space.young.page_size_bytes,
            next_offset: space.young.next_offset,
            mapped_until: space.young.mapped_until,
            allocation_alignment_bytes: space.young.allocation_alignment_bytes,
            pages,
            ranges: space.young.ranges.clone(),
            live: space.young.live.clone(),
            marked: Bitmap::with_capacity(space.young.marked.capacity()),
            local_reference_bits: space.young.local_reference_bits.clone(),
            shared_reference_bits: space.young.shared_reference_bits.clone(),
            spans: space.young.cloned_spans(),
            span_bits: space.young.span_bits.clone(),
            span_cache: space.young.span_cache.clone(),
            cursor: space.young.cursor,
            page_spans: space.young.page_spans.clone(),
            pending_range_usage: space.young.pending_range_usage,
        })
    }

    /// Fork one heap storage over operating-system copy-on-write mapping.
    fn fork_state(space: &mut Self) -> Result<Self, HeapError> {
        let small = Self::fork_small_storage(space)?;
        let large = Self::fork_large_storage(space)?;
        let young = Self::fork_young_space(space)?;
        let mapping = space.mapping.fork_lazy()?;

        Ok(Self {
            allocator: space.allocator.clone(),
            page_span_cache: PageSpanCache::new(space.allocator.pages_per_chunk()),
            max_young_allocation_bytes: space.max_young_allocation_bytes,
            young,
            small,
            large,
            page_map: Vec::new(),
            next_offset: space.next_offset,
            mapping,
            usage: space.usage,
            young_usage: space.young_usage,
            retained_page_size_bytes: space.retained_page_size_bytes,
            gc: space.gc.clone(),
            collector: super::CollectorState::default(),
        })
    }

    /// Restore one heap storage into fresh page spans.
    fn restore_state(
        allocator: Arc<Allocator>,
        image: &HeapStorageImage,
    ) -> Result<Self, HeapError> {
        let young = Self::restore_young_space(allocator.as_ref(), image)?;
        let small = Self::restore_small_storage(allocator.as_ref(), image)?;
        let large = Self::restore_large_storage(allocator.as_ref(), image)?;
        let mut mapping =
            AddressSpace::reserve(image.address_space_size_bytes(), image.page_size_bytes())?;
        restore_image_mapping(image, &mut mapping)?;
        let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
            0
        } else {
            image.max_young_allocation_bytes()
        };

        Ok(Self {
            allocator: allocator.clone(),
            page_span_cache: PageSpanCache::new(allocator.pages_per_chunk()),
            max_young_allocation_bytes,
            young,
            small,
            large,
            page_map: Vec::new(),
            next_offset: image.next_offset(),
            mapping,
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            young_usage: restored_young_usage(image),
            retained_page_size_bytes: image_retained_page_size_bytes(
                image,
                allocator.page_size_bytes(),
            ),
            gc: image.gc_state().clone(),
            collector: super::CollectorState::default(),
        })
    }

    /// Restore the heap small space from one frozen image.
    fn restore_small_storage(
        allocator: &Allocator,
        image: &HeapStorageImage,
    ) -> Result<super::SmallStorage, HeapError> {
        // restore the captured span images first
        let spans = image
            .spans()
            .iter()
            .map(|span| Self::restore_span(allocator, span))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallStorage {
            size_classes: image.size_classes().clone(),
            span_size_bytes: image.small_bytes(),
            spans: CowTable::from_vec(spans),
            partial_spans: BTreeMap::new(),
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small, allocator.page_size_bytes())?;

        Ok(small)
    }

    /// Fork the heap small space from one live space.
    fn fork_small_storage(space: &Self) -> Result<super::SmallStorage, HeapError> {
        // copy spans from the live mapping
        let spans = space
            .small
            .spans
            .iter()
            .map(|span| Self::fork_span(space, span))
            .collect::<HeapResult<Vec<_>>>()?;

        let mut small = super::SmallStorage {
            size_classes: space.small.size_classes.clone(),
            span_size_bytes: space.small.span_size_bytes,
            spans: CowTable::from_vec(spans),
            partial_spans: BTreeMap::new(),
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small, space.allocator.page_size_bytes())?;

        Ok(small)
    }

    /// Rebuild the derived reusable-span state for one restored small space.
    fn restore_partial_spans(
        small: &mut super::SmallStorage,
        page_size_bytes: usize,
    ) -> Result<(), HeapError> {
        for span_index in 0..small.spans.len() {
            let Some(span) = small.spans.get_mut(span_index) else {
                return Err(HeapError::internal("missing span"));
            };

            // validate persisted class metadata before rebuilding derived state
            span.class
                .validate(&small.size_classes, page_size_bytes, small.span_size_bytes)?;

            // rebuild the derived per-span occupancy counters
            span.occupied_count = span.occupied.count_ones();
            span.free_cursor = span.occupied.first_clear_from(0).unwrap_or(span.slot_count);

            // requeue every non-full span under its size class
            if span.occupied_count >= span.slot_count {
                continue;
            }

            small
                .partial_spans
                .entry(span.class)
                .or_default()
                .push(span_index);
        }

        Ok(())
    }

    /// Restore one heap span from one frozen span image.
    fn restore_span(allocator: &Allocator, span: &SmallSpanImage) -> HeapResult<SmallSpan> {
        // rebuild the live span around fresh pages
        let dirty_card_bytes = span.slot_count * span.class.size_class;
        let pages = allocator.allocate_pages(span.bytes.len())?;

        Ok(SmallSpan {
            first_offset: span.first_offset,
            class: span.class,
            slot_count: span.slot_count,
            occupied_count: 0,
            free_cursor: 0,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            marked: Bitmap::with_capacity(span.slot_count),
            mark_epoch: 0,
            pages,
            dirty_cards: CardSet::with_len(dirty_card_bytes),
            is_dirty_queued: false,
        })
    }

    /// Fork one heap span into shared metadata pages.
    fn fork_span(space: &Self, span: &SmallSpan) -> HeapResult<SmallSpan> {
        let pages = space.allocator.share_page_span(span.pages)?;

        Ok(SmallSpan {
            first_offset: span.first_offset,
            class: span.class,
            slot_count: span.slot_count,
            occupied_count: span.occupied_count,
            free_cursor: span.free_cursor,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            marked: Bitmap::with_capacity(span.slot_count),
            mark_epoch: 0,
            pages,
            dirty_cards: span.dirty_cards.clone(),
            is_dirty_queued: span.is_dirty_queued,
        })
    }

    /// Restore the heap large space from one frozen image.
    fn restore_large_storage(
        allocator: &Allocator,
        image: &HeapStorageImage,
    ) -> Result<super::LargeStorage, HeapError> {
        // rebuild the captured block images first
        let blocks = image
            .blocks()
            .iter()
            .map(|block| Self::restore_large_block(allocator, block))
            .collect::<HeapResult<Vec<_>>>()?;

        // rebuild the reusable block ids from the frozen table
        let free_large_block_ids = Self::free_large_block_ids(image);

        Ok(super::LargeStorage {
            blocks: CowTable::from_vec(blocks),
            free_large_block_ids,
            next_unused_large_block_id: image.next_unused_large_block_id(),
        })
    }

    /// Fork the heap large space from one live space.
    fn fork_large_storage(space: &Self) -> Result<super::LargeStorage, HeapError> {
        // copy blocks from the live mapping
        let blocks = space
            .large
            .blocks
            .iter()
            .map(|block| Self::fork_large_block(space, block))
            .collect::<HeapResult<Vec<_>>>()?;

        Ok(super::LargeStorage {
            blocks: CowTable::from_vec(blocks),
            free_large_block_ids: space.large.free_large_block_ids.clone(),
            next_unused_large_block_id: space.large.next_unused_large_block_id,
        })
    }

    /// Restore one heap block from one frozen block image.
    fn restore_large_block(
        allocator: &Allocator,
        block: &LargeBlockImage,
    ) -> HeapResult<LargeBlock> {
        let pages = if block.is_live {
            allocator.allocate_pages(block.bytes.len())?
        } else {
            PageSpan::empty()
        };

        Ok(LargeBlock {
            is_live: block.is_live,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            pages,
            trace_map: block.trace_map.clone(),
            mark_epoch: 0,
            dirty_cards: CardSet::with_len(block.byte_len),
            is_dirty_queued: false,
        })
    }

    /// Fork one heap large block into shared metadata pages.
    fn fork_large_block(space: &Self, block: &LargeBlock) -> HeapResult<LargeBlock> {
        let pages = if block.is_live {
            space.allocator.share_page_span(block.pages)?
        } else {
            PageSpan::empty()
        };

        Ok(LargeBlock {
            is_live: block.is_live,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            pages,
            trace_map: block.trace_map.clone(),
            mark_epoch: block.mark_epoch,
            dirty_cards: block.dirty_cards.clone(),
            is_dirty_queued: block.is_dirty_queued,
        })
    }

    /// Return the reusable heap block ids from one frozen table.
    fn free_large_block_ids(image: &HeapStorageImage) -> Vec<u64> {
        image
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(index, block)| (!block.is_live).then_some(index as u64 + 1))
            .collect()
    }

    /// Capture the live heap young space image.
    fn capture_young_image(&self) -> HeapResult<YoungImage> {
        let (ranges, live) = self.young.image_ranges();

        // capture the current retained bytes directly
        let bytes = self.mapping.read_bytes(0, self.young.capacity_bytes)?;

        Ok(YoungImage::new(
            self.young.capacity_bytes,
            self.young.page_size_bytes,
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
            bytes.into_boxed_slice(),
            ranges,
            self.young.cloned_spans().into_boxed_slice(),
            self.young.span_bits.clone().into_boxed_slice(),
            live,
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
        let byte_len = span.pages.len() * self.allocator.page_size_bytes();

        // capture the current retained bytes directly
        let bytes = self.mapping.read_bytes(span.first_offset, byte_len)?;

        Ok(SmallSpanImage {
            first_offset: span.first_offset,
            class: span.class,
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
            bytes: bytes.into_boxed_slice(),
        })
    }

    /// Capture every live heap block image in large space.
    fn capture_large_block_images(&self) -> HeapResult<Box<[LargeBlockImage]>> {
        self.large
            .blocks
            .iter()
            .map(|block| self.capture_large_block_image(block))
            .collect::<HeapResult<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }

    /// Capture one live heap block image in large space.
    fn capture_large_block_image(&self, block: &LargeBlock) -> HeapResult<LargeBlockImage> {
        // capture the current retained bytes directly
        let bytes = if block.is_live {
            self.mapping
                .read_bytes(block.first_offset, block.byte_len)?
                .into_boxed_slice()
        } else {
            Box::new([])
        };

        Ok(LargeBlockImage {
            is_live: block.is_live,
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            bytes,
            trace_map: block.trace_map.clone(),
        })
    }
}

/// Return the retained live page bytes in one heap storage image.
fn image_retained_page_size_bytes(image: &HeapStorageImage, page_size_bytes: usize) -> u64 {
    let young_size_bytes = retained_page_size_bytes(image.young().bytes().len(), page_size_bytes);
    let span_size_bytes = image
        .spans()
        .iter()
        .map(|span| retained_page_size_bytes(span.bytes.len(), page_size_bytes))
        .sum::<u64>();
    let allocation_bytes = image
        .blocks()
        .iter()
        .filter(|block| block.is_live)
        .map(|block| retained_page_size_bytes(block.bytes.len(), page_size_bytes))
        .sum::<u64>();

    young_size_bytes + span_size_bytes + allocation_bytes
}

/// Return the allocator-retained bytes for one restored byte range.
fn retained_page_size_bytes(byte_len: usize, page_size_bytes: usize) -> u64 {
    byte_len.div_ceil(page_size_bytes) as u64 * page_size_bytes as u64
}

/// Return the young live usage in one heap storage image.
fn restored_young_usage(image: &HeapStorageImage) -> AllocationUsage {
    let mut usage = AllocationUsage::default();

    // range blocks
    for (range_index, range) in image.young().ranges().iter().enumerate() {
        if image.young().live().contains(range_index) {
            usage.allocate(range.byte_len);
        }
    }

    // fixed-size span blocks
    for (span, bits) in image.young().spans().iter().zip(image.young().span_bits()) {
        let reserved_count = span.reserved_slot_count_with(span.next_offset);
        let freed_count = bits.freed.count_ones();
        let occupied_count = reserved_count - freed_count;

        for _ in 0..occupied_count {
            usage.allocate(span.class.size_class);
        }
    }

    usage
}

/// Restore one heap mapping from one image.
fn restore_image_mapping(image: &HeapStorageImage, mapping: &mut AddressSpace) -> HeapResult<()> {
    // restore the young mapped range first
    if !image.young().bytes().is_empty() {
        mapping.write_bytes(0, image.young().bytes())?;
    }

    // restore each captured small span range
    for span in image.spans() {
        if span.bytes.is_empty() {
            continue;
        }

        mapping.write_bytes(span.first_offset, &span.bytes)?;
    }

    // restore each captured large block range
    for block in image.blocks() {
        if !block.is_live || block.byte_len == 0 {
            continue;
        }

        mapping.write_bytes(block.first_offset, &block.bytes)?;
    }

    Ok(())
}

impl HeapStorage {
    /// Rebuild the page-map table from live storage.
    fn rebuild_page_map(&mut self) -> HeapResult<()> {
        self.page_map.clear();

        let young_pages = self.young.pages;

        self.map_page_span(0, &young_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let pages = span.pages;

            self.map_page_span(span.first_offset, &pages, |logical_page_index| {
                HeapPageMapEntry::MatureSpan {
                    span_index,
                    logical_page_index,
                }
            });
        }

        for block_index in 0..self.large.blocks.len() {
            let block_id = LargeBlockId::new(block_index as u64 + 1);
            let Some(block) = self.large_block(block_id) else {
                continue;
            };
            let pages = block.pages;

            self.map_page_span(block.first_offset, &pages, |logical_page_index| {
                HeapPageMapEntry::LargeBlock {
                    block_id,
                    logical_page_index,
                }
            });
        }

        Ok(())
    }
}
