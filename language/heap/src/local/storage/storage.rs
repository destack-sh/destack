use std::collections::BTreeMap;
use std::sync::Arc;

use destack_memory::{MemoryMap, MemoryRange};
use destack_mir::TraceMap;

use super::{
    CollectorState, GcState, HeapPlace, IndexedYoungRange, LargeBlock, LargeBlockId, PageOwner,
    SmallSpan, YoungRange, YoungSpace,
};
use crate::local::heap::HeapUsage;
use crate::{
    AllocationUsage, HeapError, HeapOptions, HeapReference, HeapResult, PageTable, ReferenceClass,
    ReferenceInput, ReferenceRange, SizeClassTable, SmallSpanClass, TraceView,
    allocation_has_reference, allocation_trace_map, slot_trace_map, visit_allocation_references,
    visit_references, visit_trace_references,
};

/// The first non-null heap large-block id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One heap storage over a shared memory.
#[derive(Debug)]
pub(crate) struct HeapStorage {
    /// The shared page memory for every heap payload.
    pub(crate) memory: Arc<MemoryMap>,
    /// The heap young space.
    pub(crate) young: YoungSpace,
    /// The heap small space.
    pub(crate) small: SmallStorage,
    /// The heap large space.
    pub(crate) large: LargeStorage,
    /// The owning heap metadata for each visible memory page.
    pub(crate) page_table: PageTable<PageOwner>,
    /// The immortal object range inside world memory, which tracing skips.
    pub(crate) immortal: MemoryRange,
    /// The exact live heap usage.
    pub(crate) usage: AllocationUsage,
    /// The exact live heap young space usage.
    pub(crate) young_usage: AllocationUsage,
    /// The exact retained memory-page bytes owned by live heap metadata.
    pub(crate) retained_bytes: u64,

    /// The maximum payload size routed to young space.
    pub(crate) max_young_allocation_bytes: usize,
    /// The completed GC cycle statistics.
    pub(crate) gc: GcState,
    /// The active collector state.
    pub(crate) collector: CollectorState,
}

impl HeapStorage {
    /// Create one heap storage from one checked options set.
    pub(crate) fn new(memory: Arc<MemoryMap>, options: &HeapOptions) -> Result<Self, HeapError> {
        // reserve one fixed young space page span up front
        let young = YoungSpace::new(
            &memory,
            options.heap_young_size_bytes,
            options.page_size_bytes,
            options.small_allocation_alignment_bytes,
        )?;
        let max_young_allocation_bytes = if options.heap_young_size_bytes == 0 {
            0
        } else {
            options.max_heap_young_allocation_size_bytes
        };

        // build the live space over the shared memory
        let young_pages = young.pages;
        let retained_bytes = young_pages.byte_len as u64;
        let mut space = Self {
            memory,
            max_young_allocation_bytes,
            young,
            small: SmallStorage {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.heap_small_size_bytes,
                spans: Vec::new(),
                partial_spans: BTreeMap::new(),
            },
            large: LargeStorage {
                blocks: Vec::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_table: PageTable::new(),
            immortal: MemoryRange::default(),
            usage: AllocationUsage::default(),
            young_usage: AllocationUsage::default(),
            retained_bytes,
            gc: GcState::default(),
            collector: CollectorState::default(),
        };

        space.map_page_span(&young_pages, |logical_page_index| PageOwner::Young {
            logical_page_index,
        });

        Ok(space)
    }

    /// Return the logical heap page size.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.young.page_size_bytes()
    }

    /// Return the exact retained heap memory-page bytes.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.retained_bytes
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
        self.usage.allocation_count() + self.young.pending_usage().allocation_count()
    }

    /// Return the number of live heap bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes() + self.young.pending_usage().allocated_bytes()
    }

    /// Return the exact live usage for this heap storage.
    pub(crate) fn usage(&self) -> HeapUsage {
        let pending_young = self.young.pending_usage();

        HeapUsage {
            allocation_count: self.usage.allocation_count() + pending_young.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes() + pending_young.allocated_bytes(),
            retained_bytes: self.retained_bytes(),
        }
    }

    /// Allocate one page-aligned memory range.
    pub(crate) fn allocate_page_span(
        &mut self,
        byte_len: usize,
        alignment: usize,
    ) -> HeapResult<MemoryRange> {
        let page_size_bytes = self.page_size_bytes();
        let byte_len = byte_len.div_ceil(page_size_bytes) * page_size_bytes;
        let alignment = alignment.max(page_size_bytes);
        let range = self.memory.allocate(byte_len, alignment)?;
        self.retained_bytes += range.byte_len as u64;

        Ok(range)
    }

    /// Release one page-aligned memory range.
    pub(crate) fn release_page_span(&mut self, range: MemoryRange) -> HeapResult<()> {
        self.memory.release(range)?;
        self.retained_bytes -= range.byte_len as u64;

        Ok(())
    }

    /// Rebuild the tracked local references that may contain shared edges.
    pub(crate) fn rebuild_shared_edge_roots(
        &mut self,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        self.collector.clear_shared_edge_roots();

        for reference in self.live_references()? {
            if !self.reference_has_shared_roots(reference, trace_view)? {
                continue;
            }

            self.collector.track_shared_edge_root(reference);
        }

        Ok(())
    }

    /// Record one pending young range block.
    pub(crate) fn record_young_range_allocation(&mut self, byte_len: usize) {
        self.young.record_range_usage(byte_len);
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
        let usage = self.young.flush_usage();
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

    /// Return one live large block by id.
    pub(crate) fn large_block(&self, block_id: LargeBlockId) -> Option<&LargeBlock> {
        // resolve the dense table slot first
        let index = block_id.index().ok()?;
        self.large.blocks.get(index)?.as_ref()
    }

    /// Return one live large block mutably by id.
    pub(crate) fn large_block_mut(&mut self, block_id: LargeBlockId) -> Option<&mut LargeBlock> {
        // resolve the dense table slot first
        let index = block_id.index().ok()?;
        self.large.blocks.get_mut(index)?.as_mut()
    }

    /// Return one live young range by object-start index.
    pub(crate) fn young_range(&self, start_index: usize) -> Option<YoungRange> {
        self.young.range(start_index)
    }

    /// Return one live young range by base offset.
    pub(crate) fn young_range_by_offset(&self, first_offset: usize) -> Option<IndexedYoungRange> {
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

    /// Return the trace map for one heap place.
    pub(crate) fn resolve_trace_map(
        &self,
        place: HeapPlace,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        match place {
            HeapPlace::YoungRange { first_offset } => self.young_range_trace_map(first_offset),
            HeapPlace::YoungSlot(slot) => self.young_slot_trace_map(slot.span_index(), trace_view),
            HeapPlace::MatureSlot(slot) => {
                self.small_slot_trace_map(slot.span_index(), slot.slot_index(), trace_view)
            }
            HeapPlace::LargeBlock(block_id) => {
                let block = self
                    .large_block(block_id)
                    .ok_or(HeapError::internal("missing large block"))?;

                Ok(block.trace_map.clone())
            }
        }
    }

    /// Visit references in one local heap allocation.
    pub(crate) fn visit_references<R: ReferenceClass>(
        &self,
        place: HeapPlace,
        trace_view: TraceView<'_>,
        input: ReferenceInput<'_>,
        range: ReferenceRange,
        visit: &mut dyn FnMut(R) -> HeapResult<()>,
    ) -> HeapResult<()> {
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(indexed) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                let byte_offset = self.young.byte_offset(indexed.range.first_offset);

                visit_allocation_references(
                    &self.young.local_reference_bits,
                    &self.young.shared_reference_bits,
                    byte_offset,
                    indexed.range.byte_len,
                    input,
                    range,
                    visit,
                )
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(span) = self.young.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                match span.class.trace_id() {
                    Some(trace_id) => {
                        visit_trace_references(trace_view, trace_id, input, range, visit)
                    }
                    None => Ok(()),
                }
            }
            HeapPlace::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                if !span.occupied.contains(slot.slot_index()) {
                    return Err(HeapError::internal("missing small slot"));
                }

                match span.class.trace_id() {
                    Some(trace_id) => {
                        visit_trace_references(trace_view, trace_id, input, range, visit)
                    }
                    None => Ok(()),
                }
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                visit_references(&block.trace_map, input, range, visit)
            }
        }
    }

    /// Return whether one local allocation may contain one reference class.
    pub(crate) fn has_reference<R: ReferenceClass>(
        &self,
        place: HeapPlace,
        trace_view: TraceView<'_>,
    ) -> HeapResult<bool> {
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(indexed) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                let byte_offset = self.young.byte_offset(indexed.range.first_offset);

                Ok(allocation_has_reference::<R>(
                    &self.young.local_reference_bits,
                    &self.young.shared_reference_bits,
                    byte_offset,
                    indexed.range.byte_len,
                    ReferenceRange::All,
                ))
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(span) = self.young.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                match span.class.trace_id() {
                    Some(trace_id) => trace_view.has_reference::<R>(trace_id),
                    None => Ok(false),
                }
            }
            HeapPlace::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                match span.class.trace_id() {
                    Some(trace_id) => trace_view.has_reference::<R>(trace_id),
                    None => Ok(false),
                }
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                Ok(R::is_present(&block.trace_map))
            }
        }
    }

    /// Return whether one allocation range may overlap one reference class.
    pub(crate) fn overlaps_reference<R: ReferenceClass>(
        &self,
        place: HeapPlace,
        trace_view: TraceView<'_>,
        range: ReferenceRange,
    ) -> HeapResult<bool> {
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(indexed) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                let byte_offset = self.young.byte_offset(indexed.range.first_offset);

                Ok(allocation_has_reference::<R>(
                    &self.young.local_reference_bits,
                    &self.young.shared_reference_bits,
                    byte_offset,
                    indexed.range.byte_len,
                    range,
                ))
            }
            HeapPlace::YoungSlot(slot) => {
                let Some(span) = self.young.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                match span.class.trace_id() {
                    Some(trace_id) => trace_view.overlaps_reference::<R>(trace_id, range),
                    None => Ok(false),
                }
            }
            HeapPlace::MatureSlot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                match span.class.trace_id() {
                    Some(trace_id) => trace_view.overlaps_reference::<R>(trace_id, range),
                    None => Ok(false),
                }
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };
                Ok(R::overlaps(&block.trace_map, range))
            }
        }
    }

    /// Return the byte length for one heap place.
    pub(crate) fn resolve_byte_len(&self, place: HeapPlace) -> HeapResult<usize> {
        match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(indexed) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };

                Ok(indexed.range.byte_len)
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

                Ok(span.class.size_class())
            }
            HeapPlace::LargeBlock(block_id) => Ok(self
                .large_block(block_id)
                .ok_or(HeapError::internal("missing large block"))?
                .byte_len),
        }
    }

    /// Return the base reference for one heap place.
    pub(crate) fn base_reference(&self, place: HeapPlace) -> HeapResult<HeapReference> {
        let base_offset = match place {
            HeapPlace::YoungRange { first_offset } => {
                let Some(indexed) = self.young_range_by_offset(first_offset) else {
                    return Err(HeapError::internal("missing young range"));
                };
                indexed.range.first_offset
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
                let slot_offset = span.class.size_class() * slot.slot_index();

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

    /// Return the exact trace map stored for one small slot.
    pub(crate) fn small_slot_trace_map(
        &self,
        span_index: usize,
        slot_index: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        let Some(span) = self.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };
        if !span.occupied.contains(slot_index) {
            return Err(HeapError::internal("missing small slot"));
        }

        // decode the table-backed class trace map
        if let Some(trace_id) = span.class.trace_id() {
            return Ok(trace_view.trace_map(trace_id)?);
        }

        Ok(slot_trace_map(
            &span.local_reference_bits,
            &span.shared_reference_bits,
            slot_index,
            span.class.size_class(),
            span.class.size_class(),
        ))
    }

    /// Return the exact trace map stored for one young span slot.
    pub(crate) fn young_slot_trace_map(
        &self,
        span_index: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        let Some(span) = self.young.span(span_index) else {
            return Err(HeapError::internal("missing span"));
        };

        // class-less young spans are no-scan
        let Some(trace_id) = span.class.trace_id() else {
            return Ok(TraceMap::Empty);
        };

        Ok(trace_view.trace_map(trace_id)?)
    }

    /// Return the exact trace map stored for one young range.
    pub(crate) fn young_range_trace_map(&self, first_offset: usize) -> HeapResult<TraceMap> {
        let Some(indexed) = self.young_range_by_offset(first_offset) else {
            return Err(HeapError::internal("missing young range"));
        };

        Ok(allocation_trace_map(
            &self.young.local_reference_bits,
            &self.young.shared_reference_bits,
            self.young.byte_offset(indexed.range.first_offset),
            indexed.range.byte_len,
        ))
    }

    /// Release memory page spans owned by this heap storage.
    fn close(&mut self) -> HeapResult<()> {
        // release every small span range
        for span in &self.small.spans {
            self.memory.release(span.pages)?;
        }

        // release every live large block range
        for block in self.large.blocks.iter().flatten() {
            self.memory.release(block.pages)?;
        }

        // release the fixed young range last
        self.memory.release(self.young.pages)?;

        Ok(())
    }
}

impl Drop for HeapStorage {
    fn drop(&mut self) {
        // abort rather than continue with corrupted local range metadata
        if self.close().is_err() {
            std::process::abort();
        }
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
    pub(crate) spans: Vec<SmallSpan>,
    /// The reusable non-full spans per exact small-span class.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
}

/// One heap large space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeStorage {
    /// The live heap blocks.
    pub(crate) blocks: Vec<Option<LargeBlock>>,
    /// The free heap block ids available for reuse.
    pub(crate) free_large_block_ids: Vec<u64>,
    /// The next heap block id to allocate.
    pub(crate) next_unused_large_block_id: u64,
}
