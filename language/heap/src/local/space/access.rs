use destack_mir::{TraceMap, TraceTable};

use super::{HeapExtent, HeapSpace, HeapStorage};
use crate::{
    HeapError, HeapReference, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference,
    scan_references,
};

impl HeapSpace {
    /// Return the base native address for direct heap access.
    #[inline(always)]
    pub fn base_address(&self) -> usize {
        self.mapping.base_address()
    }

    /// Return whether one heap reference currently refers to one live block.
    pub fn is_live(&self, reference: HeapReference) -> bool {
        self.resolve_extent(reference).is_some()
    }

    /// Return whether one heap reference currently refers to young space.
    #[cfg(test)]
    pub(crate) fn is_young(&self, reference: HeapReference) -> bool {
        matches!(
            self.resolve_extent(reference).map(|extent| extent.storage),
            Some(HeapStorage::YoungRange { .. }) | Some(HeapStorage::YoungSlot(_))
        )
    }

    /// Return the live storage for one heap reference.
    #[cfg(test)]
    pub(crate) fn storage(&self, reference: HeapReference) -> Option<HeapStorage> {
        Some(self.resolve_extent(reference)?.storage)
    }

    /// Return the trace map for one heap reference.
    pub fn scan(&self, reference: HeapReference, trace_table: &TraceTable) -> HeapResult<TraceMap> {
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_heap_reference(reference));
        };

        self.trace_map_for_place(extent.storage, trace_table)
    }

    /// Record one heap write barrier for one live heap block.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.record_write_barrier(reference, extent, byte_offset, byte_len, trace_table)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        let (extent, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.shared_write_barrier_extent_bytes(extent, byte_offset, bytes, trace_table)
    }

    /// Return old and new shared edges for one already-resolved heap store.
    fn shared_write_barrier_extent_bytes(
        &self,
        extent: HeapExtent,
        byte_offset: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        // skip ranges that cannot contain shared references
        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;
        if !self.overlaps_shared_roots(&trace_map, byte_offset, bytes.len()) {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();

        // overwritten references
        let base_address = self.mapping.base_address() + extent.base.offset();
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, bytes.len()),
            &mut edges,
        )?;

        // inserted references
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::bytes(byte_offset, bytes),
            ReferenceRange::bytes(byte_offset, bytes.len()),
            &mut edges,
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
        let byte_offset = block_byte_offset(extent.byte_offset, start, byte_len, extent.byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Record all local-heap metadata affected by one write.
    pub(crate) fn record_write_barrier(
        &mut self,
        reference: HeapReference,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // local collector metadata
        self.record_local_write(extent, byte_offset, byte_len, trace_table)?;

        // shared collector metadata
        self.record_shared_edge_write(reference, extent, byte_offset, byte_len, trace_table)
    }

    /// Record local collector metadata for one live heap extent.
    fn record_local_write(
        &mut self,
        extent: HeapExtent,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        self.write_major_barrier(extent, byte_offset, byte_len, trace_table)?;

        // only mature extents need remembered-write bookkeeping
        match extent.storage {
            HeapStorage::YoungRange { .. } | HeapStorage::YoungSlot(_) => Ok(()),
            HeapStorage::MatureSlot(slot) => {
                let trace_map =
                    self.small_slot_trace_map(slot.span_index(), slot.slot_index(), trace_table)?;

                self.mark_span_slot_dirty(
                    slot.span_index(),
                    slot.slot_index(),
                    byte_offset,
                    byte_len,
                    &trace_map,
                )
            }
            HeapStorage::LargeBlock(block_id) => {
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
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // inactive local-to-shared scan
        if !self.collector.is_scanning_shared_edges {
            return Ok(());
        }

        // skip writes that cannot touch shared references
        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;
        if !self.overlaps_shared_roots(&trace_map, byte_offset, byte_len) {
            return Ok(());
        }

        self.queue_shared_reference(reference, trace_table)
    }
}

/// Return one block-local byte offset for one visible range.
fn block_byte_offset(
    base_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    debug_assert!(base_offset <= capacity);

    // validate the caller start relative to the visible payload
    let remaining = capacity - base_offset;
    if start > remaining {
        return Err(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        });
    }

    // validate the caller length after projecting the start
    let byte_offset = base_offset + start;
    let remaining = capacity - byte_offset;
    if len > remaining {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        });
    }

    Ok(byte_offset)
}
