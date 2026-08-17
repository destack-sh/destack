use crate::TraceView;
use destack_mir::TraceMap;
use std::sync::Arc;

use super::{HeapExtent, HeapPlace, HeapStorage, LargeBlockId};
use crate::{
    DropPlan, HeapError, HeapResult, ReferenceClass, ReferenceInput, ReferenceRange,
    SharedHeapReference, visit_references, visit_trace_references,
};

impl HeapStorage {
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

    /// Publish shared edges from one already-resolved byte range.
    fn publish_extent_edges(
        &self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // inactive collector
        let Some(_publication) = self.gc.begin_mark_publication() else {
            return Ok(());
        };

        // empty writes cannot publish references
        if byte_len == 0 {
            return Ok(());
        }

        // scan inserted shared references in mapped heap memory
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, byte_len),
            &mut |reference| self.mark_reference(None, reference),
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
