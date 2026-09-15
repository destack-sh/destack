use crate::TraceView;
use destack_mir::TraceMap;

use super::{HeapExtent, HeapPlace, HeapStorage};
use crate::{
    DropPlan, HeapEdge, HeapError, HeapReference, HeapResult, ReferenceInput, ReferenceRange,
    SharedHeapReference, visit_heap_edges,
};

impl HeapStorage {
    /// Return the base native address for direct heap access.
    #[inline(always)]
    pub(crate) fn base_address(&self) -> usize {
        self.memory.base_address()
    }

    /// Return whether one heap reference currently refers to one live block.
    pub(crate) fn is_live(&self, reference: HeapReference) -> bool {
        self.resolve_extent(reference).is_some()
    }

    /// Return the drop plan for one live allocation base.
    pub(crate) fn drop_plan(&self, reference: HeapReference) -> HeapResult<Option<DropPlan>> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };
        if extent.byte_offset != 0 {
            return Err(HeapError::invalid_heap_reference(reference));
        }

        match extent.place {
            HeapPlace::Slot(slot) => self
                .span(slot.span_index())
                .map(|span| span.class.drop_plan())
                .ok_or_else(|| HeapError::internal("missing span")),
            HeapPlace::LargeBlock(block_id) => self
                .large_block(block_id)
                .map(|block| block.drop)
                .ok_or_else(|| HeapError::internal("missing large block")),
        }
    }

    /// Zero one byte range in a live heap allocation.
    pub(crate) fn zero(&self, reference: HeapReference, byte_len: usize) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, 0, byte_len)?;
        let offset = extent.base.offset() + byte_offset;

        self.memory.zero(offset, byte_len).map_err(HeapError::from)
    }

    /// Return the live place for one heap reference.
    #[cfg(test)]
    pub(crate) fn place(&self, reference: HeapReference) -> Option<HeapPlace> {
        Some(self.resolve_extent(reference)?.place)
    }

    /// Return the trace map for one heap reference.
    pub(crate) fn trace_map(
        &self,
        reference: HeapReference,
        trace_view: TraceView<'_>,
    ) -> HeapResult<TraceMap> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        self.resolve_trace_map(extent.place, trace_view)
    }

    /// Record one heap write barrier for one live heap block.
    pub(crate) fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.record_write_barrier(reference, extent, byte_offset, byte_len, trace_view)
    }

    /// Retain the allocations one payload references for the collector.
    pub(crate) fn retain_payload(&mut self, trace_map: &TraceMap, bytes: &[u8]) -> HeapResult<()> {
        let mut references = Vec::new();
        visit_heap_edges(trace_map, 0, bytes, ReferenceRange::All, &mut |edge| {
            if let HeapEdge::Local(reference) = edge {
                references.push(reference);
            }

            Ok(())
        })?;
        for reference in references {
            self.retain(reference)?;
        }

        Ok(())
    }

    /// Retain one allocation for the collector while a borrow or heap storage reaches it.
    pub(crate) fn retain(&mut self, reference: HeapReference) -> HeapResult<()> {
        if reference.is_nullish() || self.is_constant(reference) {
            return Ok(());
        }
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        match extent.place {
            HeapPlace::Slot(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                span.retained.set(slot.slot_index());
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };
                block.retained = true;
            }
        }

        Ok(())
    }

    /// Record that one retained allocation's values moved out, so the collector frees it alone.
    pub(crate) fn mark_empty(&mut self, reference: HeapReference) -> HeapResult<()> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        match extent.place {
            HeapPlace::Slot(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };
                span.empty.set(slot.slot_index());
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };
                block.empty = true;
            }
        }

        Ok(())
    }

    /// Return the logical byte length of one allocation base.
    pub(crate) fn byte_len(&self, reference: HeapReference) -> HeapResult<usize> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        Ok(extent.byte_len)
    }

    /// Return whether the managed graph retained one allocation.
    pub(crate) fn is_retained(&self, reference: HeapReference) -> HeapResult<bool> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        Ok(match extent.place {
            HeapPlace::Slot(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::internal("missing span"));
                };

                span.retained.contains(slot.slot_index())
            }
            HeapPlace::LargeBlock(block_id) => {
                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                block.retained
            }
        })
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub(crate) fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        let (extent, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.shared_write_barrier_extent_bytes(extent, byte_offset, bytes, trace_view)
    }

    /// Return old and new shared edges for one already-resolved heap store.
    fn shared_write_barrier_extent_bytes(
        &self,
        extent: HeapExtent,
        byte_offset: usize,
        bytes: &[u8],
        trace_view: TraceView<'_>,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        // skip ranges that cannot contain shared references
        let range = ReferenceRange::bytes(byte_offset, bytes.len());
        if !self.overlaps_reference::<SharedHeapReference>(extent.place, trace_view, range)? {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();

        // overwritten references
        let base_address = self.memory.base_address() + extent.base.offset();
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::mapped(base_address),
            range,
            &mut |reference| {
                edges.push(reference);

                Ok(())
            },
        )?;

        // inserted references
        self.visit_references::<SharedHeapReference>(
            extent.place,
            trace_view,
            ReferenceInput::bytes(byte_offset, bytes),
            range,
            &mut |reference| {
                edges.push(reference);

                Ok(())
            },
        )?;

        // publish only real shared references
        edges.retain(|reference| !reference.is_nullish());

        Ok(edges)
    }

    /// Return one checked live extent and byte offset for one heap range.
    fn resolve_range(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(HeapExtent, usize)> {
        // resolve live block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        // project caller range into the block payload
        let byte_offset = extent.project(start, byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Record all local-heap metadata affected by one write.
    pub(crate) fn record_write_barrier(
        &mut self,
        reference: HeapReference,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // local collector metadata
        self.write_mark_barrier(extent, byte_offset, byte_len, trace_view)?;

        // shared collector metadata
        self.record_shared_edge_write(reference, extent, byte_offset, byte_len, trace_view)
    }

    /// Record local-to-shared edge metadata for one live heap extent.
    fn record_shared_edge_write(
        &mut self,
        reference: HeapReference,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        // inactive local-to-shared scan
        if !self.collector.is_scanning_shared_edges {
            return Ok(());
        }

        // skip writes that cannot touch shared references
        let range = ReferenceRange::bytes(byte_offset, byte_len);
        let is_overlapping =
            self.overlaps_reference::<SharedHeapReference>(extent.place, trace_view, range)?;
        if !is_overlapping {
            return Ok(());
        }

        self.queue_shared_reference(reference, trace_view)
    }
}
