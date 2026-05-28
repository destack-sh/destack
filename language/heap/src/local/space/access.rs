use destack_mir::{TraceMap, TraceTable};

use super::{HeapLocation, HeapPlace, HeapSpace, YoungPlace};
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

    /// Return whether one heap reference currently refers to one live allocation.
    pub fn is_live(&self, reference: HeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    /// Return whether one heap reference currently refers to young space.
    #[cfg(test)]
    pub(crate) fn is_young(&self, reference: HeapReference) -> bool {
        matches!(
            self.resolve_location(reference)
                .map(|location| location.place),
            Some(HeapPlace::Young(YoungPlace::Range { .. }))
                | Some(HeapPlace::Young(YoungPlace::Slot(_)))
        )
    }

    /// Return the live place for one heap reference.
    #[cfg(test)]
    pub(crate) fn place(&self, reference: HeapReference) -> Option<HeapPlace> {
        Some(self.resolve_location(reference)?.place)
    }

    /// Return the trace map for one heap reference.
    pub fn scan(&self, reference: HeapReference, trace_table: &TraceTable) -> HeapResult<TraceMap> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        self.trace_map_for_place(location.place, trace_table)
    }

    /// Record one heap write barrier for one live heap allocation.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.record_write_barrier(reference, location, byte_offset, byte_len, trace_table)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        let (location, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.shared_write_barrier_location_bytes(location, byte_offset, bytes, trace_table)
    }

    /// Return old and new shared edges for one already-resolved heap store.
    fn shared_write_barrier_location_bytes(
        &self,
        location: HeapLocation,
        byte_offset: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<Vec<SharedHeapReference>> {
        // skip ranges that cannot contain shared references
        let trace_map = self.trace_map_for_place(location.place, trace_table)?;
        if !self.overlaps_shared_roots(&trace_map, byte_offset, bytes.len()) {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();

        // overwritten references
        let base_address = self.mapping.base_address() + location.base.offset();
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

    /// Return one checked live location and byte offset for one heap range.
    fn resolve_range(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(HeapLocation, usize)> {
        // resolve live allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        // project caller range into the allocation payload
        let byte_offset =
            allocation_byte_offset(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Record all local-heap metadata affected by one write.
    pub(crate) fn record_write_barrier(
        &mut self,
        reference: HeapReference,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // local collector metadata
        self.record_local_write(location, byte_offset, byte_len, trace_table)?;

        // shared collector metadata
        self.record_shared_edge_write(reference, location, byte_offset, byte_len, trace_table)
    }

    /// Record local collector metadata for one live heap location.
    fn record_local_write(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        self.write_major_barrier(location, byte_offset, byte_len, trace_table)?;

        // only mature locations need remembered-write bookkeeping
        match location.place {
            HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_)) => {
                Ok(())
            }
            HeapPlace::Small(slot) => {
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
            HeapPlace::Large(allocation_id) => {
                self.mark_large_allocation_dirty(allocation_id, byte_offset, byte_len)
            }
        }
    }

    /// Record local-to-shared edge metadata for one live heap location.
    fn record_shared_edge_write(
        &mut self,
        reference: HeapReference,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        // inactive local-to-shared scan
        if !self.collector.is_scanning_shared_edges {
            return Ok(());
        }

        // skip writes that cannot touch shared references
        let trace_map = self.trace_map_for_place(location.place, trace_table)?;
        if !self.overlaps_shared_roots(&trace_map, byte_offset, byte_len) {
            return Ok(());
        }

        self.queue_shared_reference(reference, trace_table)
    }
}

/// Return one allocation-local byte offset for one visible range.
fn allocation_byte_offset(
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
