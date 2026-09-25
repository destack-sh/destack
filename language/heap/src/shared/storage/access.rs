use crate::TraceView;
use std::sync::Arc;
use tspp_mir::TraceMap;

use parking_lot::RwLock;

use super::{HeapExtent, HeapPlace, HeapStorage, LargeBlock, LargeBlockId, SmallSpan};
use crate::{
    AllocationCache, DropPlan, HeapEdge, HeapError, HeapResult, ReferenceClass, ReferenceInput,
    ReferenceRange, SharedHeapReference, visit_heap_edges, visit_references,
    visit_trace_references,
};

impl HeapStorage {
    /// Return the logical byte length of one live shared allocation base.
    pub(crate) fn byte_len(&self, reference: SharedHeapReference) -> HeapResult<usize> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        if extent.byte_offset != 0 {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        }

        Ok(extent.byte_len)
    }

    /// Return the drop plan for one live shared allocation base.
    pub(crate) fn drop_plan(&self, reference: SharedHeapReference) -> HeapResult<Option<DropPlan>> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        if extent.byte_offset != 0 {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        }

        match extent.place {
            HeapPlace::SmallSlot(slot) => {
                let store = self.state.read();
                let span = store
                    .small
                    .spans
                    .get(slot.span_index())
                    .ok_or_else(|| HeapError::internal("missing shared span"))?;

                Ok(span.class.drop_plan())
            }
            HeapPlace::LargeBlock(block_id) => {
                let store = self.state.read();
                let index = block_id.index()?;
                let block = store
                    .large
                    .blocks
                    .get(index)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| HeapError::internal("missing shared large block"))?;
                let block = block.read();

                Ok(block.drop)
            }
        }
    }

    /// Retain the shared allocations one payload references.
    pub(crate) fn retain_payload(
        &self,
        cache: &mut AllocationCache,
        trace_map: &TraceMap,
        bytes: &[u8],
    ) -> HeapResult<()> {
        visit_heap_edges(trace_map, 0, bytes, ReferenceRange::All, &mut |edge| {
            if let HeapEdge::Shared(reference) = edge
                && !reference.is_nullish()
                && !self.is_constant(reference)
            {
                self.flush_reference_cache(cache, reference);
                self.retain(reference)?;
            }

            Ok(())
        })
    }

    /// Retain one allocation for the collector while a borrow or heap storage reaches it.
    pub(crate) fn retain(&self, reference: SharedHeapReference) -> HeapResult<()> {
        if reference.is_nullish() || self.is_constant(reference) {
            return Ok(());
        }

        self.access_base(
            reference,
            |span, slot_index| span.retained.set(slot_index),
            |block| block.write().retained = true,
        )
    }

    /// Record that one retained allocation's values moved out, so the collector frees it alone.
    pub(crate) fn mark_empty(&self, reference: SharedHeapReference) -> HeapResult<()> {
        self.access_base(
            reference,
            |span, slot_index| span.empty.set(slot_index),
            |block| block.write().empty = true,
        )
    }

    /// Return whether a borrow or heap storage retained one allocation.
    pub(crate) fn is_retained(&self, reference: SharedHeapReference) -> HeapResult<bool> {
        self.access_base(
            reference,
            |span, slot_index| span.retained.contains(slot_index),
            |block| block.read().retained,
        )
    }

    /// Access the span slot or large block holding one shared allocation.
    fn access_base<T>(
        &self,
        reference: SharedHeapReference,
        slot: impl FnOnce(&SmallSpan, usize) -> T,
        block: impl FnOnce(&RwLock<LargeBlock>) -> T,
    ) -> HeapResult<T> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };
        let store = self.state.read();

        match extent.place {
            HeapPlace::SmallSlot(place) => {
                let span = store
                    .small
                    .spans
                    .get(place.span_index())
                    .ok_or_else(|| HeapError::internal("missing shared span"))?;

                Ok(slot(span, place.slot_index()))
            }
            HeapPlace::LargeBlock(block_id) => {
                let index = block_id.index()?;
                let large = store
                    .large
                    .blocks
                    .get(index)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| HeapError::internal("missing shared large block"))?;

                Ok(block(large))
            }
        }
    }

    /// Zero one byte range in a live shared heap allocation.
    pub(crate) fn zero(&self, reference: SharedHeapReference, byte_len: usize) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, 0, byte_len)?;
        let offset = extent.base.offset() + byte_offset;

        self.memory.zero(offset, byte_len).map_err(HeapError::from)
    }

    /// Return the trace map for one shared heap reference.
    pub(crate) fn trace_map(
        &self,
        reference: SharedHeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        let (extent, _) = self.resolve_range(reference, 0, 0)?;

        match extent.place {
            HeapPlace::SmallSlot(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                let Some(trace_id) = span.class.trace_id() else {
                    return Ok(TraceMap::Empty);
                };

                Ok(trace_view.trace_map(trace_id)?)
            }
            HeapPlace::LargeBlock(block_id) => {
                let trace_map = self.large_block_trace_map(block_id)?;
                let trace_map = (*trace_map).clone();

                Ok(trace_map)
            }
        }
    }

    /// Record one shared heap write barrier before one byte store.
    pub(crate) fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.write_shared_barrier_bytes(reference, byte_offset, bytes, trace_view)
    }

    /// Record one shared heap write barrier after one completed byte store.
    pub(crate) fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.publish_extent_edges(extent, byte_offset, byte_len, trace_view)
    }

    /// Retain the shared edges one resolved byte range stores.
    fn publish_extent_edges(
        &self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // skip a write of no bytes
        if byte_len == 0 {
            return Ok(());
        }

        // scan inserted shared references in mapped heap memory
        let publication = self.gc.begin_mark_publication();
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, byte_len),
            &mut |reference| {
                self.retain(reference)?;
                if publication.is_some() {
                    self.mark_reference(None, reference)?;
                }

                Ok(())
            },
        )
    }

    /// Return one checked live extent and byte offset for one shared heap range.
    fn resolve_range(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(HeapExtent, usize)> {
        // resolve live block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // project caller range into the block payload
        let byte_offset = extent.project(start, byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Visit references in one shared heap allocation.
    pub(crate) fn visit_references<R: ReferenceClass>(
        &self,
        place: HeapPlace,
        trace_view: TraceView<'_>,
        input: ReferenceInput<'_>,
        range: ReferenceRange,
        visit: &mut dyn FnMut(R) -> HeapResult<()>,
    ) -> HeapResult<()> {
        match place {
            HeapPlace::SmallSlot(slot) => {
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                let trace_id = span.class.trace_id();
                drop(store);

                match trace_id {
                    Some(trace_id) => {
                        visit_trace_references(trace_view, trace_id, input, range, visit)
                    }
                    None => Ok(()),
                }
            }
            HeapPlace::LargeBlock(block_id) => {
                let trace_map = self.large_block_trace_map(block_id)?;

                visit_references(&trace_map, input, range, visit)
            }
        }
    }

    /// Return the immutable trace map for one shared large block.
    pub(crate) fn large_block_trace_map(
        &self,
        block_id: LargeBlockId,
    ) -> HeapResult<Arc<TraceMap>> {
        let store = self.state.read();
        let Some(block) = store
            .large
            .blocks
            .get(block_id.index()?)
            .and_then(Option::as_ref)
            .cloned()
        else {
            return Err(HeapError::internal("missing large block"));
        };
        drop(store);
        let trace_map = block.read().trace_map.clone();

        Ok(trace_map)
    }
}
