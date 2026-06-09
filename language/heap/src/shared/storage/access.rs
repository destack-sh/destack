use destack_mir::{TraceMap, TraceTable};
use std::borrow::Cow;

use super::storage::block_byte_offset;
use super::{HeapExtent, HeapPlace, HeapStorage};
use crate::{
    HeapError, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference, visit_references,
};

impl HeapStorage {
    /// Return the trace map for one shared heap reference.
    pub(crate) fn trace_map(
        &self,
        reference: SharedHeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        let (extent, _) = self.resolve_range(reference, 0, 0)?;
        let trace_map = self.trace_map_for_place_ref(extent.storage, trace_table)?;

        Ok(trace_map.into_owned())
    }

    /// Record one shared heap write barrier before one byte store.
    pub(crate) fn write_barrier_bytes(
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
    pub(crate) fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
        trace_table: &TraceTable,
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.publish_extent_edges(extent, byte_offset, byte_len, trace_table)
    }

    /// Publish shared edges from one already-resolved byte range.
    fn publish_extent_edges(
        &self,
        extent: HeapExtent,
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
        let trace_map = self.trace_map_for_place_ref(extent.storage, trace_table)?;
        let base_address = self.mapping.base_address() + extent.base.offset();
        visit_references::<SharedHeapReference>(
            &trace_map,
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
        let byte_offset = block_byte_offset(extent.byte_offset, start, byte_len, extent.byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Return the trace map for one shared heap extent.
    pub(crate) fn trace_map_for_place_ref<'a>(
        &self,
        storage: HeapPlace,
        trace_table: &'a TraceTable,
    ) -> HeapResult<Cow<'a, TraceMap>> {
        // dispatch by physical shared heap storage
        match storage {
            HeapPlace::SmallSlot(slot) => {
                self.small_slot_trace_map_ref(slot.span_index(), slot.slot_index(), trace_table)
            }
            HeapPlace::LargeBlock(block_id) => {
                let trace_map = self
                    .state
                    .read()
                    .large
                    .blocks
                    .get(block_id.index()?)
                    .cloned()
                    .ok_or(HeapError::internal("missing large block"))?
                    .read()
                    .trace_map
                    .clone();

                Ok(Cow::Owned(trace_map))
            }
        }
    }
}
