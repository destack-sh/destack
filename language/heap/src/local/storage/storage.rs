use std::collections::BTreeMap;
use std::sync::Arc;

use tspp_memory::{MemoryMap, MemoryRange};
use tspp_mir::TraceMap;

use super::{HeapPlace, LargeBlock, LargeBlockId, PageOwner, SmallSpan};
use crate::local::gc::CollectorState;
use crate::local::heap::HeapUsage;
use crate::{
    AllocationUsage, GcState, HeapError, HeapOptions, HeapResult, PageTable, ReferenceClass,
    ReferenceInput, ReferenceRange, SizeClassTable, SmallSpanClass, TraceView, slot_trace_map,
    visit_references, visit_trace_references,
};

/// The first non-null heap large-block id.
const FIRST_ALLOCATED_LARGE_ALLOCATION_ID: u64 = 1;

/// One heap storage over a shared memory.
#[derive(Debug)]
pub(crate) struct HeapStorage {
    /// The shared page memory for every heap payload.
    pub(crate) memory: Arc<MemoryMap>,
    /// The heap small space.
    pub(crate) small: SmallStorage,
    /// The heap large space.
    pub(crate) large: LargeStorage,
    /// The owning heap metadata for each visible memory page.
    pub(crate) page_table: PageTable<PageOwner>,
    /// The constant object range inside world memory, which tracing skips.
    pub(crate) constant: MemoryRange,
    /// The exact live heap usage.
    pub(crate) usage: AllocationUsage,
    /// The exact retained memory-page bytes owned by live heap metadata.
    pub(crate) retained_bytes: u64,
    /// The logical heap page size.
    pub(crate) page_size_bytes: usize,

    /// The completed GC cycle statistics.
    pub(crate) gc: GcState,
    /// The active collector state.
    pub(crate) collector: CollectorState,
}

impl HeapStorage {
    /// Create one heap storage from one checked options set.
    pub(crate) fn new(memory: Arc<MemoryMap>, options: &HeapOptions) -> Result<Self, HeapError> {
        Ok(Self {
            memory,
            small: SmallStorage {
                size_classes: options.size_classes.clone(),
                span_size_bytes: options.heap_small_size_bytes,
                spans: Vec::new(),
                partial_spans: BTreeMap::new(),
                cursors: Vec::new(),
            },
            large: LargeStorage {
                blocks: Vec::new(),
                free_large_block_ids: Vec::new(),
                next_unused_large_block_id: FIRST_ALLOCATED_LARGE_ALLOCATION_ID,
            },
            page_table: PageTable::new(),
            constant: MemoryRange::default(),
            usage: AllocationUsage::default(),
            retained_bytes: 0,
            page_size_bytes: options.page_size_bytes,
            gc: GcState::default(),
            collector: CollectorState::default(),
        })
    }

    /// Return the logical heap page size.
    pub(crate) const fn page_size_bytes(&self) -> usize {
        self.page_size_bytes
    }

    /// Return the exact retained heap memory-page bytes.
    pub(crate) fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    /// Return the current GC state.
    pub(crate) fn gc_state(&self) -> &GcState {
        &self.gc
    }

    /// Return the number of live heap blocks.
    pub(crate) fn allocation_count(&self) -> usize {
        self.usage.allocation_count()
    }

    /// Return the number of live heap bytes.
    pub(crate) fn allocated_bytes(&self) -> u64 {
        self.usage.allocated_bytes()
    }

    /// Return the exact live usage for this heap storage.
    pub(crate) fn usage(&self) -> HeapUsage {
        HeapUsage {
            allocation_count: self.usage.allocation_count(),
            allocated_bytes: self.usage.allocated_bytes(),
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
            HeapPlace::Slot(slot) => {
                self.slot_trace_map(slot.span_index(), slot.slot_index(), trace_view)
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
            HeapPlace::Slot(slot) => {
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
            HeapPlace::Slot(slot) => {
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
            HeapPlace::Slot(slot) => {
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
            HeapPlace::Slot(slot) => {
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

    /// Return the exact trace map stored for one span slot.
    pub(crate) fn slot_trace_map(
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

    /// Release memory page spans owned by this heap storage.
    fn close(&mut self) -> HeapResult<()> {
        // release every live span range
        for span in &self.small.spans {
            self.memory.release(span.pages)?;
        }

        // release every live large block range
        for block in self.large.blocks.iter().flatten() {
            self.memory.release(block.pages)?;
        }

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
    /// The reusable non-full spans per exact small-span class, excluding the class cursor.
    pub(crate) partial_spans: BTreeMap<SmallSpanClass, Vec<usize>>,
    /// The span each small allocation class reserves from, by class cache index.
    pub(crate) cursors: Vec<Option<usize>>,
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
