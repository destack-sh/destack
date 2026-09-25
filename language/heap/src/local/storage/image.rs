use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_memory::{MemoryMap, MemoryRange};
use tspp_serde::Reflect;

use super::{
    HeapStorage, LargeBlock, LargeBlockId, LargeBlockImage, PageOwner, SmallSpan, SmallSpanImage,
};
use crate::local::gc::CollectorState;
use crate::{
    AllocationUsage, Bitmap, GcState, HeapCaptureBlocker, HeapError, HeapResult, PageTable,
    SizeClassTable, TraceView,
};

/// One frozen local heap storage metadata image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct HeapStorageImage {
    /// The configured local page width.
    pub(crate) page_size_bytes: usize,
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured small-space span width.
    pub(crate) small_bytes: usize,
    /// The captured heap spans.
    pub(crate) spans: Box<[SmallSpanImage]>,
    /// The captured heap blocks in large space.
    pub(crate) blocks: Box<[Option<LargeBlockImage>]>,
    /// The captured reusable large-block ids.
    pub(crate) free_large_block_ids: Box<[u64]>,

    /// The next heap block id to allocate in large space.
    pub(crate) next_unused_large_block_id: u64,
    /// The number of allocated heap references.
    pub(crate) allocated_count: usize,
    /// The number of allocated heap bytes.
    pub(crate) allocated_bytes: u64,

    /// The captured GC state.
    pub(crate) gc_state: GcState,
}

impl HeapStorageImage {
    /// Return the retained heap page count represented by this image.
    pub(crate) fn page_count(&self) -> usize {
        let span_pages = self
            .spans
            .iter()
            .map(|span| span.class.span_size_bytes().div_ceil(self.page_size_bytes))
            .sum::<usize>();
        let block_pages = self
            .blocks
            .iter()
            .flatten()
            .map(|block| block.byte_len.div_ceil(self.page_size_bytes))
            .sum::<usize>();

        span_pages + block_pages
    }

    /// Return the retained memory bytes represented by this image.
    fn retained_bytes(&self) -> u64 {
        self.page_count() as u64 * self.page_size_bytes as u64
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

        let mut space = Self::fork_state(self, memory)?;

        // rebuild derived shared-edge state after fork
        space.rebuild_shared_edge_roots(trace_view)?;

        Ok(space)
    }

    /// Restore one heap storage from one frozen heap storage image.
    pub(crate) fn from_image(
        memory: Arc<MemoryMap>,
        image: &HeapStorageImage,
        trace_view: TraceView<'_>,
    ) -> Result<Self, HeapError> {
        let mut space = Self::restore_state(memory.clone(), image)?;

        // rebuild the page table and the shared-edge roots after restore
        space
            .rebuild_page_table()
            .and_then(|()| space.rebuild_shared_edge_roots(trace_view))?;

        Ok(space)
    }

    /// Return one frozen heap storage image.
    pub(crate) fn image(&mut self) -> Result<HeapStorageImage, HeapError> {
        self.check_branch_boundary()?;

        // capture the live heap blocks directly
        let spans = self.capture_span_images();
        let blocks = self.capture_large_block_images();

        // freeze the current heap image
        Ok(HeapStorageImage {
            page_size_bytes: self.page_size_bytes(),
            size_classes: self.small.size_classes.clone(),
            small_bytes: self.small.span_size_bytes,
            spans,
            blocks,
            free_large_block_ids: self.large.free_large_block_ids.clone().into_boxed_slice(),
            next_unused_large_block_id: self.large.next_unused_large_block_id,
            allocated_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
            gc_state: self.gc.clone(),
        })
    }

    /// Check that image and fork boundaries cannot capture transient GC state.
    pub(crate) fn check_branch_boundary(&self) -> Result<(), HeapError> {
        if self.collector.is_collecting() {
            return Err(HeapError::capture_blocked(HeapCaptureBlocker::GcActive));
        }

        Ok(())
    }

    /// Fork one heap storage over operating-system copy-on-write memory.
    fn fork_state(space: &mut Self, memory: Arc<MemoryMap>) -> Result<Self, HeapError> {
        let small = Self::fork_small_storage(space)?;
        let large = Self::fork_large_storage(space);
        Ok(Self {
            memory,
            small,
            large,
            page_table: space.page_table.clone(),
            constant: space.constant,
            usage: space.usage,
            retained_bytes: space.retained_bytes,
            page_size_bytes: space.page_size_bytes,
            gc: space.gc.clone(),
            collector: CollectorState::default(),
        })
    }

    /// Restore heap storage metadata over captured world memory.
    fn restore_state(memory: Arc<MemoryMap>, image: &HeapStorageImage) -> Result<Self, HeapError> {
        let small = Self::restore_small_storage(image)?;
        let large = Self::restore_large_storage(image);
        let retained_bytes = image.retained_bytes();

        Ok(Self {
            memory,
            small,
            large,
            page_table: PageTable::new(),
            constant: MemoryRange::default(),
            usage: AllocationUsage::new(image.allocated_count, image.allocated_bytes),
            retained_bytes,
            page_size_bytes: image.page_size_bytes,
            gc: image.gc_state.clone(),
            collector: CollectorState::default(),
        })
    }

    /// Restore the heap small space from one frozen image.
    fn restore_small_storage(image: &HeapStorageImage) -> Result<super::SmallStorage, HeapError> {
        // restore the captured span images first
        let spans = image.spans.iter().map(Self::restore_span).collect();

        let mut small = super::SmallStorage {
            size_classes: image.size_classes.clone(),
            span_size_bytes: image.small_bytes,
            spans,
            partial_spans: BTreeMap::new(),
            cursors: Vec::new(),
        };

        // rebuild the derived span occupancy state
        Self::restore_partial_spans(&mut small, image.page_size_bytes)?;

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
            cursors: Vec::new(),
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
            retained: span.retained.clone(),
            empty: span.empty.clone(),
            mark_epoch: 0,
            pages,
        }
    }

    /// Fork one heap span into shared metadata pages.
    fn fork_span(span: &SmallSpan) -> SmallSpan {
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
            retained: span.retained.clone(),
            empty: span.empty.clone(),
            mark_epoch: 0,
            pages: span.pages,
        }
    }

    /// Restore the heap large space from one frozen image.
    fn restore_large_storage(image: &HeapStorageImage) -> super::LargeStorage {
        // rebuild the captured block images first
        let blocks = image
            .blocks
            .iter()
            .map(|block| Self::restore_large_block(image.page_size_bytes, block))
            .collect();

        super::LargeStorage {
            blocks,
            free_large_block_ids: image.free_large_block_ids.to_vec(),
            next_unused_large_block_id: image.next_unused_large_block_id,
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
            retained: block.retained,
            empty: block.empty,
            mark_epoch: 0,
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
            retained: block.retained,
            empty: block.empty,
            mark_epoch: block.mark_epoch,
        })
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
            retained: span.retained.clone(),
            empty: span.empty.clone(),
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
            retained: block.retained,
            empty: block.empty,
        }
    }
}

impl HeapStorage {
    /// Rebuild the page table from live storage.
    fn rebuild_page_table(&mut self) -> HeapResult<()> {
        self.page_table.clear_all();

        for span_index in 0..self.small.spans.len() {
            let Some(span) = self.span(span_index) else {
                return Err(HeapError::internal("missing span"));
            };
            let pages = span.pages;

            self.map_page_span(&pages, |logical_page_index| PageOwner::Span {
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
