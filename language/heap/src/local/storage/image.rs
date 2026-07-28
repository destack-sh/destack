use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::{MemoryMap, MemoryRange};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{
    CardSet, GcState, HeapStorage, LargeBlock, LargeBlockId, LargeBlockImage, PageOwner, SmallSpan,
    SmallSpanImage, YoungImage, YoungSpace,
};
use crate::{
    AllocationClass, AllocationUsage, Bitmap, HeapAllocationError, HeapCaptureBlocker,
    HeapConfigurationError, HeapError, HeapResult, PageTable, SizeClassTable, TraceView,
};

/// One frozen local heap storage metadata image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct HeapStorageImage {
    /// The captured branchable young space image.
    young: YoungImage,

    /// The configured size-class table.
    size_classes: SizeClassTable,
    /// The configured small-space span width.
    small_bytes: usize,
    /// The captured heap spans.
    spans: Box<[SmallSpanImage]>,
    /// The captured heap blocks in large space.
    blocks: Box<[Option<LargeBlockImage>]>,
    /// The captured reusable large-block ids.
    free_large_block_ids: Box<[u64]>,

    /// The configured maximum payload size routed to young space.
    max_young_allocation_bytes: usize,
    /// The next heap block id to allocate in large space.
    next_unused_large_block_id: u64,
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
        blocks: Box<[Option<LargeBlockImage>]>,
        free_large_block_ids: Box<[u64]>,
        max_young_allocation_bytes: usize,
        next_unused_large_block_id: u64,
        allocated_count: usize,
        allocated_bytes: u64,
        gc_state: GcState,
    ) -> Self {
        Self {
            young,
            size_classes,
            small_bytes,
            spans,
            blocks,
            free_large_block_ids,
            max_young_allocation_bytes,
            next_unused_large_block_id,
            allocated_count,
            allocated_bytes,
            gc_state,
        }
    }

    /// Return the branchable young space image.
    pub(crate) fn young(&self) -> &YoungImage {
        &self.young
    }

    /// Return the configured local page width.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.young.page_size_bytes()
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

    /// Return the captured heap blocks in large space.
    pub(crate) fn blocks(&self) -> &[Option<LargeBlockImage>] {
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

    /// Return the allocated heap reference count.
    pub(crate) const fn allocated_count(&self) -> usize {
        self.allocated_count
    }

    /// Return the allocated heap bytes.
    pub(crate) const fn allocated_bytes(&self) -> u64 {
        self.allocated_bytes
    }

    /// Return the retained heap page count represented by this image.
    pub(crate) fn page_count(&self) -> usize {
        let page_size_bytes = self.page_size_bytes();
        let young_pages = self.young().capacity_bytes().div_ceil(page_size_bytes);
        let span_pages = self
            .spans()
            .iter()
            .map(|span| span.class.span_size_bytes().div_ceil(page_size_bytes))
            .sum::<usize>();
        let block_pages = self
            .blocks()
            .iter()
            .flatten()
            .map(|block| block.byte_len.div_ceil(page_size_bytes))
            .sum::<usize>();

        young_pages + span_pages + block_pages
    }

    /// Return the captured collector state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc_state
    }

    /// Return the retained memory bytes represented by this image.
    fn retained_bytes(&self) -> u64 {
        self.page_count() as u64 * self.page_size_bytes() as u64
    }

    /// Return the live young allocation usage represented by this image.
    fn young_usage(&self) -> AllocationUsage {
        let mut usage = AllocationUsage::default();

        // count live range blocks
        for (range_index, range) in self.young().ranges().iter().enumerate() {
            if self.young().live().contains(range_index) {
                usage.allocate(range.byte_len);
            }
        }

        // count live fixed-size span blocks
        for (span, bits) in self.young().spans().iter().zip(self.young().span_bits()) {
            let reserved_count = span.reserved_slot_count_with(span.next_offset);
            let freed_count = bits.freed.count_ones();
            let occupied_count = reserved_count - freed_count;

            usage.allocate_many(
                occupied_count,
                occupied_count as u64 * span.class.size_class() as u64,
            );
        }

        usage
    }
}

impl HeapStorage {
    /// Fork one heap storage over the same shared memory.
    ///
    /// Call this only from a safepoint where the heap storage cannot mutate.
    pub(crate) fn fork(
        &mut self,
        memory: Arc<MemoryMap>,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        self.check_branch_boundary()?;
        self.flush_young_cursor();

        let mut space = Self::fork_state(self, memory)?;

        // rebuild derived remembered-set state after fork
        space
            .rebuild_remembered_set(trace_view)
            .and_then(|()| space.rebuild_shared_edge_roots(trace_view))?;

        Ok(space)
    }

    /// Restore one heap storage from one frozen heap storage image.
    pub(crate) fn from_image(
        memory: Arc<MemoryMap>,
        image: &HeapStorageImage,
        trace_view: TraceView<'_>,
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

        Self::restore_from_image(memory, image, trace_view)
    }

    /// Restore one heap storage from one checked frozen heap storage image.
    fn restore_from_image(
        memory: Arc<MemoryMap>,
        image: &HeapStorageImage,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        let mut space = Self::restore_state(memory.clone(), image)?;

        // rebuild remembered-set state conservatively after restore
        space
            .rebuild_page_table()
            .and_then(|()| space.rebuild_remembered_set(trace_view))
            .and_then(|()| space.rebuild_shared_edge_roots(trace_view))?;

        Ok(space)
    }

    /// Return one frozen heap storage image.
    pub(crate) fn image(&mut self) -> Result<HeapStorageImage, HeapError> {
        self.check_branch_boundary()?;
        self.flush_young_cursor();

        // capture the live heap blocks directly
        let young = self.capture_young_image();
        let spans = self.capture_span_images();
        let blocks = self.capture_large_block_images();

        // freeze the current heap image
        Ok(HeapStorageImage::new(
            young,
            self.small.size_classes.clone(),
            self.small.span_size_bytes,
            spans,
            blocks,
            self.large.free_large_block_ids.clone().into_boxed_slice(),
            self.max_young_allocation_bytes,
            self.large.next_unused_large_block_id,
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
    fn restore_young_space(image: &HeapStorageImage) -> HeapResult<YoungSpace> {
        let pages = MemoryRange {
            offset: image.young().memory_offset(),
            byte_len: image.young().capacity_bytes(),
        };
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
            if span.byte_len() == 0 || span.byte_len() > span.class().size_class() {
                return Err(HeapError::invalid_allocation(
                    HeapAllocationError::ByteLengthMismatch {
                        expected: span.class().size_class(),
                        actual: span.byte_len(),
                    },
                ));
            }

            let class = AllocationClass::select(
                span.byte_len(),
                image.young().allocation_alignment_bytes(),
                span.class().trace_id(),
                span.class().drop_plan(),
                span.class().is_noscan(),
                image.size_classes(),
                image.young().page_size_bytes(),
                image.small_bytes(),
            );
            let Some(small) = class.as_small() else {
                return Err(HeapError::invalid_allocation(
                    HeapAllocationError::ByteLengthMismatch {
                        expected: span.class().size_class(),
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

            let page_start = (span.first_offset - pages.offset) / image.young().page_size_bytes();
            let page_count = span.span_size_bytes() / image.young().page_size_bytes();
            for page_span in page_spans.iter_mut().skip(page_start).take(page_count) {
                *page_span = Some(span_index);
            }
        }
        Ok(YoungSpace {
            capacity_bytes: image.young().capacity_bytes(),
            page_size_bytes: image.young().page_size_bytes(),
            next_offset: image.young().next_offset(),
            mapped_until: pages.offset + image.young().capacity_bytes(),
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
        let pages = space.young.pages;

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

    /// Fork one heap storage over operating-system copy-on-write memory.
    fn fork_state(space: &mut Self, memory: Arc<MemoryMap>) -> Result<Self, HeapError> {
        let small = Self::fork_small_storage(space)?;
        let large = Self::fork_large_storage(space);
        let young = Self::fork_young_space(space)?;
        Ok(Self {
            memory,
            max_young_allocation_bytes: space.max_young_allocation_bytes,
            young,
            small,
            large,
            page_table: space.page_table.clone(),
            usage: space.usage,
            young_usage: space.young_usage,
            retained_bytes: space.retained_bytes,
            gc: space.gc.clone(),
            collector: super::CollectorState::default(),
        })
    }

    /// Restore heap storage metadata over captured world memory.
    fn restore_state(memory: Arc<MemoryMap>, image: &HeapStorageImage) -> Result<Self, HeapError> {
        let young = Self::restore_young_space(image)?;
        let small = Self::restore_small_storage(image)?;
        let large = Self::restore_large_storage(image);
        let retained_bytes = image.retained_bytes();
        let max_young_allocation_bytes = if image.young().capacity_bytes() == 0 {
            0
        } else {
            image.max_young_allocation_bytes()
        };

        Ok(Self {
            memory,
            max_young_allocation_bytes,
            young,
            small,
            large,
            page_table: PageTable::new(),
            usage: AllocationUsage::new(image.allocated_count(), image.allocated_bytes()),
            young_usage: image.young_usage(),
            retained_bytes,
            gc: image.gc_state().clone(),
            collector: super::CollectorState::default(),
        })
    }

    /// Restore the heap small space from one frozen image.
    fn restore_small_storage(image: &HeapStorageImage) -> Result<super::SmallStorage, HeapError> {
        // restore the captured span images first
        let spans = image.spans().iter().map(Self::restore_span).collect();

        let mut small = super::SmallStorage {
            size_classes: image.size_classes().clone(),
            span_size_bytes: image.small_bytes(),
            spans,
            partial_spans: BTreeMap::new(),
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small, image.young().page_size_bytes())?;

        Ok(small)
    }

    /// Fork the heap small space from one live space.
    fn fork_small_storage(space: &Self) -> Result<super::SmallStorage, HeapError> {
        // copy spans from the live memory
        let spans = space.small.spans.iter().map(Self::fork_span).collect();

        let mut small = super::SmallStorage {
            size_classes: space.small.size_classes.clone(),
            span_size_bytes: space.small.span_size_bytes,
            spans,
            partial_spans: BTreeMap::new(),
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small, space.page_size_bytes())?;

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
    fn restore_span(span: &SmallSpanImage) -> SmallSpan {
        // rebuild the live span around its captured page range
        let dirty_card_bytes = span.slot_count * span.class.size_class();
        let pages = MemoryRange {
            offset: span.first_offset,
            byte_len: span.class.span_size_bytes(),
        };
        SmallSpan {
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
        }
    }

    /// Fork one heap span into shared metadata pages.
    fn fork_span(span: &SmallSpan) -> SmallSpan {
        let pages = span.pages;

        SmallSpan {
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
        }
    }

    /// Restore the heap large space from one frozen image.
    fn restore_large_storage(image: &HeapStorageImage) -> super::LargeStorage {
        // rebuild the captured block images first
        let blocks = image
            .blocks()
            .iter()
            .map(|block| Self::restore_large_block(image.page_size_bytes(), block))
            .collect();

        super::LargeStorage {
            blocks,
            free_large_block_ids: image.free_large_block_ids.to_vec(),
            next_unused_large_block_id: image.next_unused_large_block_id(),
        }
    }

    /// Fork the heap large space from one live space.
    fn fork_large_storage(space: &Self) -> super::LargeStorage {
        // copy blocks from the live memory
        let blocks = space
            .large
            .blocks
            .iter()
            .map(Self::fork_large_block)
            .collect();

        super::LargeStorage {
            blocks,
            free_large_block_ids: space.large.free_large_block_ids.clone(),
            next_unused_large_block_id: space.large.next_unused_large_block_id,
        }
    }

    /// Restore one heap block from one frozen block image.
    fn restore_large_block(
        page_size_bytes: usize,
        block: &Option<LargeBlockImage>,
    ) -> Option<LargeBlock> {
        let Some(block) = block else {
            return None;
        };
        let pages = MemoryRange {
            offset: block.first_offset,
            byte_len: block.byte_len.next_multiple_of(page_size_bytes),
        };
        Some(LargeBlock {
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            pages,
            trace_map: block.trace_map.clone(),
            drop: block.drop,
            mark_epoch: 0,
            dirty_cards: CardSet::with_len(block.byte_len),
            is_dirty_queued: false,
        })
    }

    /// Fork one heap large block into shared metadata pages.
    fn fork_large_block(block: &Option<LargeBlock>) -> Option<LargeBlock> {
        let Some(block) = block else {
            return None;
        };

        Some(LargeBlock {
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            pages: block.pages,
            trace_map: block.trace_map.clone(),
            drop: block.drop,
            mark_epoch: block.mark_epoch,
            dirty_cards: block.dirty_cards.clone(),
            is_dirty_queued: block.is_dirty_queued,
        })
    }

    /// Capture the live heap young space image.
    fn capture_young_image(&self) -> YoungImage {
        let (ranges, live) = self.young.image_ranges();

        YoungImage::new(
            self.young.pages.offset,
            self.young.capacity_bytes,
            self.young.page_size_bytes,
            self.young.next_offset,
            self.young.allocation_alignment_bytes,
            ranges,
            self.young.cloned_spans().into_boxed_slice(),
            self.young.span_bits.clone().into_boxed_slice(),
            live,
            self.young.local_reference_bits.clone(),
            self.young.shared_reference_bits.clone(),
        )
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
            first_offset: span.first_offset,
            class: span.class,
            slot_count: span.slot_count,
            occupied: span.occupied.clone(),
            local_reference_bits: span.local_reference_bits.clone(),
            shared_reference_bits: span.shared_reference_bits.clone(),
        }
    }

    /// Capture every live heap block image in large space.
    fn capture_large_block_images(&self) -> Box<[Option<LargeBlockImage>]> {
        self.large
            .blocks
            .iter()
            .map(|block| block.as_ref().map(Self::capture_large_block_image))
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }

    /// Capture one live heap block image in large space.
    fn capture_large_block_image(block: &LargeBlock) -> LargeBlockImage {
        LargeBlockImage {
            first_offset: block.first_offset,
            byte_len: block.byte_len,
            trace_map: block.trace_map.clone(),
            drop: block.drop,
        }
    }
}

impl HeapStorage {
    /// Rebuild the page table from live storage.
    fn rebuild_page_table(&mut self) -> HeapResult<()> {
        self.page_table.clear_all();

        let young_pages = self.young.pages;

        self.map_page_span(&young_pages, |logical_page_index| PageOwner::Young {
            logical_page_index,
        });

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let pages = span.pages;

            self.map_page_span(&pages, |logical_page_index| PageOwner::MatureSpan {
                span_index,
                logical_page_index,
            });
        }

        for block_index in 0..self.large.blocks.len() {
            let block_id = LargeBlockId::new(block_index as u64 + 1);
            let Some(block) = self.large_block(block_id) else {
                continue;
            };
            let pages = block.pages;

            self.map_page_span(&pages, |logical_page_index| PageOwner::LargeBlock {
                block_id,
                logical_page_index,
            });
        }

        Ok(())
    }
}
