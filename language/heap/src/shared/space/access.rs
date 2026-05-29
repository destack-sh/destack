use destack_mir::{TraceMap, TraceTable};

use super::space::{allocation_byte_offset, small_slot_offset};
use super::{SharedHeapPlace, SharedHeapRegion, SharedHeapSpace};
use crate::{
    HeapError, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference, scan_references,
};

impl SharedHeapSpace {
    /// Fill one caller-provided buffer from one shared heap allocation at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (region, byte_offset) = self.resolve_range(reference, start, target.len())?;

        self.fill_region_bytes(region, byte_offset, target)
    }

    /// Return the trace map for one shared heap reference.
    pub fn scan(
        &self,
        reference: SharedHeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        let (region, _) = self.resolve_range(reference, 0, 0)?;

        self.trace_map_for_place(region.place, trace_table)
    }

    /// Record one shared heap write barrier before one byte store.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.write_shared_barrier_bytes(reference, byte_offset, bytes, trace_table)
    }

    /// Record one shared heap write barrier after one completed byte store.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let (region, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.publish_region_edges(region, byte_offset, byte_len, trace_table)
    }

    /// Publish shared edges from one already-resolved byte range.
    fn publish_region_edges(
        &self,
        region: SharedHeapRegion,
        byte_offset: usize,
        byte_len: usize,
        trace_table: &TraceTable,
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
        let trace_map = self.trace_map_for_place(region.place, trace_table)?;
        let mut edges = Vec::new();

        let base_address = self.mapping.base_address() + region.base.offset();
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, byte_len),
            &mut edges,
        )?;

        // publish discovered references
        self.queue_references(None, edges)
    }

    /// Return one checked live region and byte offset for one shared heap range.
    fn resolve_range(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedHeapRegion, usize)> {
        // resolve live allocation
        let Some(region) = self.resolve_region(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // project caller range into the allocation payload
        let byte_offset =
            allocation_byte_offset(region.byte_offset, start, byte_len, region.byte_len)?;

        Ok((region, byte_offset))
    }

    /// Return the trace map for one shared heap region.
    pub(crate) fn trace_map_for_place(
        &self,
        place: SharedHeapPlace,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        // dispatch by physical shared heap place
        match place {
            SharedHeapPlace::Small(slot) => {
                self.small_slot_trace_map(slot.span_index(), slot.slot_index(), trace_table)
            }
            SharedHeapPlace::Large(allocation_id) => {
                let trace_map = self
                    .state
                    .read()
                    .large
                    .allocations
                    .get(allocation_id.index()?)
                    .cloned()
                    .ok_or(HeapError::internal("missing large allocation"))?
                    .read()
                    .trace_map
                    .clone();

                Ok(trace_map)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared heap region.
    fn fill_region_bytes(
        &self,
        region: SharedHeapRegion,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the owning place
        match region.place {
            SharedHeapPlace::Small(slot) => {
                // resolve the small span
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::internal("missing span"));
                };
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = slot_offset + byte_offset;

                Ok(self
                    .mapping
                    .read_bytes_into(span.first_offset + read_offset, target)?)
            }
            SharedHeapPlace::Large(allocation_id) => {
                // resolve the large allocation
                let store = self.state.read();
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::internal("missing large allocation"));
                };
                let allocation = allocation.read();

                if !allocation.is_live {
                    return Err(HeapError::internal("missing large allocation"));
                }

                self.mapping
                    .read_bytes_into(allocation.first_offset + byte_offset, target)
                    .map_err(HeapError::from)
            }
        }
    }
}
