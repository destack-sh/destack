use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::AddressSpace;
use destack_mir::{TraceMap, TraceTable};

use super::{
    CollectorState, GcState, HeapPageMapEntry, HeapPlace, LargeBlock, LargeBlockId, SmallSpan,
    YoungRange, YoungSpace,
};
use crate::allocator::{Allocator, PageSpan, PageSpanCache, SizeClassTable};
use crate::local::heap::HeapUsage;
use crate::{
    AllocationPlan, AllocationShape, AllocationUsage, CowTable, HeapError, HeapOptions,
    HeapReference, HeapResult, SmallSpanClass, allocation_plan, allocation_trace_map,
    slot_trace_map,
};

/// The first non-null heap large-block id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One heap storage over a shared allocator.
#[derive(Debug)]
pub(crate) struct HeapStorage {
    /// The shared page allocator for every heap payload.
    pub(super) allocator: Arc<Allocator>,
    /// The local cache of reusable page spans.
    pub(crate) page_span_cache: PageSpanCache,

    /// The heap young space.
    pub(crate) young: YoungSpace,
    /// The heap small space.
    pub(crate) small: SmallStorage,
    /// The heap large space.
    pub(crate) large: LargeStorage,
    /// The owning heap metadata for each visible allocator page.
    pub(crate) page_map: Vec<Option<HeapPageMapEntry>>,
    /// The next unused byte offset in heap storage.
    pub(crate) next_offset: usize,
    /// The fixed live byte mapping for heap storage.
    pub(crate) mapping: AddressSpace,
    /// The exact live heap usage.
    pub(crate) usage: AllocationUsage,
    /// The exact live heap young space usage.
    pub(crate) young_usage: AllocationUsage,
    /// The exact retained allocator-page bytes owned by live heap metadata.
    pub(crate) retained_page_size_bytes: u64,

    /// The maximum payload size routed to young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The completed GC cycle summary.
    pub(crate) gc: GcState,
    /// The active collector state.
    pub(crate) collector: CollectorState,
}

impl HeapStorage {
    /// Create one heap storage from one checked options set.
    pub(crate) fn build_with_options(
        allocator: Arc<Allocator>,
        options: &HeapOptions,
    ) -> Result<Self, HeapError> {
        let mut page_span_cache = PageSpanCache::new(allocator.pages_per_chunk());
        let mapping =
            AddressSpace::reserve(options.address_space_size_bytes, options.page_size_bytes)?;

        // reserve one fixed young space page span up front
        let young = YoungSpace::new(
            &allocator,
            options.heap_young_size_bytes,
            options.page_size_bytes,
            options.small_allocation_alignment_bytes,
            &mut page_span_cache,
        )?;
        let max_young_allocation_bytes = if options.heap_young_size_bytes == 0 {
            0
        } else {
            options.max_heap_young_allocation_size_bytes
        };

        let next_heap_offset = options.heap_young_size_bytes.max(options.page_size_bytes);
        let next_heap_offset = align_up(next_heap_offset, options.page_size_bytes);

        // build the live space over the shared allocator
        let young_pages = young.pages;
        let retained_page_size_bytes = young_pages.len() as u64 * options.page_size_bytes as u64;
        let mut space = Self {
            allocator,
            page_span_cache,
            max_young_allocation_bytes,
            young,
            small: SmallStorage {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.heap_small_size_bytes,
                spans: CowTable::new(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeStorage {
                blocks: CowTable::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_map: Vec::new(),
            next_offset: next_heap_offset,
            mapping,
            usage: AllocationUsage::default(),
            young_usage: AllocationUsage::default(),
            retained_page_size_bytes,
            gc: GcState::default(),
            collector: CollectorState::default(),
        };

        space.map_page_span(0, &young_pages, |logical_page_index| {
            HeapPageMapEntry::Young { logical_page_index }
        });

        Ok(space)
    }

    /// Return the shared page allocator.
    pub(crate) fn allocator(&self) -> &Arc<Allocator> {
        &self.allocator
    }

    /// Return the exact retained heap allocator-page bytes.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.retained_page_size_bytes
            + self
                .page_span_cache
                .cached_bytes(self.allocator.page_size_bytes())
    }

    /// Return the current GC state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc
    }

    /// Validate one heap reference for stable scoped access.
    pub(crate) fn stabilize(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        let Some(_extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        Ok(reference)
    }

    /// Pin one heap reference against movement.
    pub(crate) fn pin(&mut self, reference: HeapReference) -> HeapResult<HeapReference> {
        // first validate the reference against live heap state
        let reference = self.stabilize(reference)?;

        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // then record the active pin count
        self.collector.pins.pin(extent.base)?;

        Ok(reference)
    }

    /// Release one heap pin.
    pub(crate) fn unpin(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        self.collector.pins.unpin(extent.base)
    }

    /// Return the number of live heap blocks.
    pub(crate) fn allocation_count(&self) -> usize {
        self.usage.allocation_count() + self.young.pending_cursor_usage().allocation_count()
    }

    /// Return the number of live heap bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes() + self.young.pending_cursor_usage().allocated_bytes()
    }

    /// Return the exact live usage for this heap storage.
    pub(crate) fn usage(&self) -> HeapUsage {
        let pending_young = self.young.pending_cursor_usage();

        HeapUsage {
            allocation_count: self.usage.allocation_count() + pending_young.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes() + pending_young.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Flush transient cache state before one exact branch boundary.
    pub(crate) fn flush_branch_boundary(&mut self) -> HeapResult<()> {
        self.flush_young_cursor();
        self.page_span_cache.flush(&self.allocator)
    }

    /// Allocate one page span through the local page-span cache.
    pub(crate) fn allocate_page_span(&mut self, byte_len: usize) -> HeapResult<PageSpan> {
        let page_span = self
            .page_span_cache
            .allocate_pages(&self.allocator, byte_len)?;
        self.retained_page_size_bytes += self.page_span_size_bytes(page_span);

        Ok(page_span)
    }

    /// Release one page span through the local page-span cache.
    pub(crate) fn release_page_span(&mut self, page_span: PageSpan) -> HeapResult<()> {
        self.page_span_cache
            .release_page_span(&self.allocator, page_span)?;
        self.retained_page_size_bytes -= self.page_span_size_bytes(page_span);

        Ok(())
    }

    /// Rebuild the tracked local references that may contain shared edges.
    pub(crate) fn rebuild_shared_edge_roots(&mut self, trace_table: &TraceTable) -> HeapResult<()> {
        self.collector.clear_shared_edge_roots();

        for reference in self.live_references()? {
            if !self.reference_has_shared_roots(reference, trace_table)? {
                continue;
            }

            self.collector.track_shared_edge_root(reference);
        }

        Ok(())
    }

    /// Record one young heap block.
    pub(crate) fn record_young_allocation(&mut self, byte_len: usize) {
        self.usage.allocate(byte_len);
        self.young_usage.allocate(byte_len);
    }

    /// Record multiple young heap blocks.
    pub(crate) fn record_young_allocations(&mut self, usage: AllocationUsage) {
        let allocation_count = usage.allocation_count();
        let allocated_bytes = usage.allocated_bytes();

        if allocation_count == 0 {
            return;
        }

        self.usage.allocate_many(allocation_count, allocated_bytes);
        self.young_usage
            .allocate_many(allocation_count, allocated_bytes);
    }

    /// Flush the active young cursor into exact usage.
    #[inline(always)]
    pub(crate) fn flush_young_cursor(&mut self) {
        let usage = self.young.flush_cursor();
        self.record_young_allocations(usage);
    }

    /// Record one mature heap block.
    pub(crate) fn record_mature_allocation(&mut self, byte_len: usize) {
        self.usage.allocate(byte_len);
    }

    /// Record one young heap block free.
    pub(crate) fn record_young_free(&mut self, byte_len: usize) {
        let freed_bytes = byte_len as u64;

        self.usage.free(freed_bytes);
        self.young_usage.free(freed_bytes);
    }

    /// Record one mature heap block free.
    pub(crate) fn record_mature_free(&mut self, byte_len: usize) {
        self.usage.free(byte_len as u64);
    }

    /// Return the retained byte width for one page span.
    fn page_span_size_bytes(&self, page_span: PageSpan) -> u64 {
        page_span.len() as u64 * self.allocator.page_size_bytes() as u64
    }

    /// Return one live large block by id.
    pub(crate) fn large_block(&self, block_id: LargeBlockId) -> Option<&LargeBlock> {
        // resolve the dense table slot first
        let index = block_id.index().ok()?;
        let block = self.large.blocks.get(index)?;

        // skip free blocks
        block.is_live.then_some(block)
    }

    /// Return one live large block mutably by id.
    pub(crate) fn large_block_mut(&mut self, block_id: LargeBlockId) -> Option<&mut LargeBlock> {
        // resolve the dense table slot first
        let index = block_id.index().ok()?;
        let block = self.large.blocks.get_mut(index)?;

        // skip free blocks
        block.is_live.then_some(block)
    }

    /// Return one live young range by object-start index.
    pub(crate) fn young_range(&self, start_index: usize) -> Option<YoungRange> {
        self.young.range(start_index)
    }

    /// Return one live young range by base offset.
    pub(crate) fn young_range_by_offset(&self, first_offset: usize) -> Option<(usize, YoungRange)> {
        self.young.range_by_offset(first_offset)
    }

    /// Return one live heap span by index.
    pub(crate) fn span(&self, span_index: usize) -> Option<&SmallSpan> {
        self.small.spans.get(span_index)
    }

    /// Return one live heap span mutably by index.
    pub(crate) fn span_mut(&mut self, span_index: usize) -> Option<&mut SmallSpan> {
        self.small.spans.get_mut(span_index)
    }

    /// Return the trace map for one heap storage.
    pub(crate) fn trace_map_for_place(
        &self,
        storage: HeapPlace,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        match storage {
            HeapPlace::YoungRange { first_offset } => self.young_range_trace_map(first_offset),
            HeapPlace::YoungSlot(slot) => self.young_slot_trace_map(slot.span_index(), trace_table),
            HeapPlace::MatureSlot(slot) => {
                self.small_slot_trace_map(slot.span_index(), slot.slot_index(), trace_table)
            }
            HeapPlace::LargeBlock(block_id) => {
                let trace_map = self
                    .large_block(block_id)
                    .ok_or(HeapError::internal("missing large block"))?
                    .trace_map
                    .clone();

                Ok(trace_map)
            }
        }
    }

    /// Return the byte length for one heap storage.
    pub(crate) fn byte_len_for_place(&self, storage: HeapPlace) -> HeapResult<usize> {
        match storage {
            HeapPlace::YoungRange { first_offset } => {
                let Some((_range_index, range)) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                Ok(range.byte_len)
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(span) = self.young.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                Ok(span.byte_len())
            }
            HeapPlace::MatureSlot(slot) => {
                let span = self
                    .span(slot.span_index())
                    .ok_or(HeapError::internal("missing span"))?;

                if !span.occupied.contains(slot.slot_index()) {
                    return Err(HeapError::internal("missing small slot"));
                }

                Ok(span.class.size_class)
            }
            HeapPlace::LargeBlock(block_id) => Ok(self
                .large_block(block_id)
                .ok_or(HeapError::internal("missing large block"))?
                .byte_len),
        }
    }

    /// Return the base reference for one heap storage.
    pub(crate) fn base_reference(&self, storage: HeapPlace) -> HeapResult<HeapReference> {
        let base_offset = match storage {
            HeapPlace::YoungRange { first_offset } => {
                let Some((_allocation_index, block)) = self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::internal("missing young range"));
                };
                block.first_offset
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(span) = self.young.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.slot_offset(slot.slot_index())
            }
            HeapPlace::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                let slot_offset = span.class.size_class * slot.slot_index();

                span.first_offset + slot_offset
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.first_offset
            }
        };

        Ok(HeapReference::new(base_offset))
    }

    /// Reserve one logical heap storage byte range.
    pub(crate) fn reserve_address_range(&mut self, byte_len: usize) -> HeapResult<usize> {
        self.reserve_address_range_aligned(byte_len, self.allocator.page_size_bytes())
    }

    /// Reserve one logical heap storage byte range with the given alignment.
    pub(crate) fn reserve_address_range_aligned(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<usize> {
        debug_assert!(self.next_offset <= self.mapping.byte_len());

        let alignment = alignment.max(self.allocator.page_size_bytes());
        let first_offset = align_up(self.next_offset, alignment);
        let next_offset = first_offset + byte_len;
        if next_offset > self.mapping.byte_len() {
            return Err(HeapError::InvalidByteRange {
                start: first_offset,
                len: byte_len,
                capacity: self.mapping.byte_len(),
            });
        }

        self.next_offset = next_offset;

        Ok(first_offset)
    }

    /// Return the exact trace map stored for one small slot.
    pub(crate) fn small_slot_trace_map(
        &self,
        span_index: usize,
        slot_index: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::internal("missing small slot"));
        }

        if let Some(trace_id) = span.class.trace_id {
            let trace_map = trace_table
                .trace(trace_id)
                .ok_or(HeapError::internal("missing trace map"))?;

            return Ok(trace_map.clone());
        }

        Ok(slot_trace_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class,
            span.class.size_class,
        ))
    }

    /// Return the exact trace map stored for one young span slot.
    pub(crate) fn young_slot_trace_map(
        &self,
        span_index: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        let Some(span) = self.young.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        let Some(trace_id) = span.class.trace_id else {
            return Ok(TraceMap::Empty);
        };

        let trace_map = trace_table
            .trace(trace_id)
            .ok_or(HeapError::internal("missing trace map"))?;

        Ok(trace_map.clone())
    }

    /// Return the exact trace map stored for one young range.
    pub(crate) fn young_range_trace_map(&self, first_offset: usize) -> HeapResult<TraceMap> {
        let Some((_range_index, range)) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::internal("missing young range"));
        };
        Ok(allocation_trace_map(
            &self.young.local_reference_bits,
            &self.young.shared_reference_bits,
            range.first_offset,
            range.byte_len,
        ))
    }

    /// Resolve one allocation shape against this heap storage.
    #[inline(always)]
    pub(crate) fn allocation_plan<'a>(&self, shape: AllocationShape<'a>) -> AllocationPlan<'a> {
        allocation_plan(
            shape,
            &self.small.size_classes,
            self.allocator.page_size_bytes(),
            self.small.span_size_bytes,
        )
    }

    /// Return the page spans reachable from this live heap storage.
    pub(crate) fn live_page_spans(&self) -> Vec<PageSpan> {
        let mut page_spans = Vec::new();

        // collect span page spans first
        page_spans.extend(self.small.spans.iter().map(|span| span.pages));

        // collect large-block page spans next
        page_spans.extend(self.large.blocks.iter().map(|block| block.pages));

        // collect the young space page span last
        page_spans.push(self.young.pages);

        page_spans
    }

    /// Release allocator page spans owned by this heap storage.
    fn close(&mut self) -> HeapResult<()> {
        for page_span in self.live_page_spans() {
            self.release_page_span(page_span)?;
        }

        self.page_span_cache.flush(&self.allocator)
    }
}

impl Drop for HeapStorage {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

/// One heap small space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SmallStorage {
    /// The configured size-class table.
    pub(crate) size_classes: SizeClassTable,
    /// The configured span width.
    pub(crate) span_size_bytes: usize,
    /// The live heap spans.
    pub(crate) spans: CowTable<SmallSpan>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

/// One heap large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeStorage {
    /// The live heap blocks.
    pub(crate) blocks: CowTable<LargeBlock>,
    /// The free heap block ids available for reuse.
    pub(crate) free_large_block_ids: Vec<u64>,
    /// The next heap block id to allocate.
    pub(crate) next_unused_large_block_id: u64,
}

/// Return the offset rounded up to one block boundary.
fn align_up(byte_len: usize, alignment_bytes: usize) -> usize {
    let alignment_bytes = alignment_bytes.max(1);

    byte_len.div_ceil(alignment_bytes) * alignment_bytes
}
