use crate::TraceView;
use destack_mir::TraceMap;

use super::{HeapExtent, HeapPlace, HeapStorage};
use crate::{
    HeapError, HeapReference, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference,
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

    /// Return whether one heap reference currently refers to young space.
    #[cfg(test)]
    pub(crate) fn is_young(&self, reference: HeapReference) -> bool {
        matches!(
            self.resolve_extent(reference).map(|extent| extent.place),
            Some(HeapPlace::YoungRange { .. }) | Some(HeapPlace::YoungSlot(_))
        )
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
        edges.retain(|reference| !reference.is_null());

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
        self.record_local_write(extent, byte_offset, byte_len, trace_view)?;

        // shared collector metadata
        self.record_shared_edge_write(reference, extent, byte_offset, byte_len, trace_view)
    }

    /// Record local collector metadata for one live heap extent.
    fn record_local_write(
        &mut self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_view: TraceView<'_>,
    ) -> HeapResult<()> {
        self.write_major_barrier(extent, byte_offset, byte_len, trace_view)?;

        // only mature extents need remembered-write bookkeeping
        match extent.place {
            HeapPlace::YoungRange { .. } | HeapPlace::YoungSlot(_) => Ok(()),
            HeapPlace::MatureSlot(slot) => {
                // skip writes that cannot touch local references
                let range = ReferenceRange::bytes(byte_offset, byte_len);
                let is_overlapping = self.overlaps_reference::<HeapReference>(
                    HeapPlace::MatureSlot(slot),
                    trace_view,
                    range,
                )?;
                if !is_overlapping {
                    return Ok(());
                }

                self.remember_span_slot_write(
                    slot.span_index(),
                    slot.slot_index(),
                    byte_offset,
                    byte_len,
                )
            }
            HeapPlace::LargeBlock(block_id) => {
                self.mark_large_block_dirty(block_id, byte_offset, byte_len)
            }
        }
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
