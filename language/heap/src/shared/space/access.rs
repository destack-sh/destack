use destack_mir::{TraceMap, TraceTable};

use super::space::{block_byte_offset, small_slot_offset};
use super::{SharedHeapExtent, SharedHeapSpace, SharedHeapStorage};
use crate::{
    HeapError, HeapResult, ReferenceInput, ReferenceRange, SharedHeapReference, scan_references,
};

impl SharedHeapSpace {
    /// Fill one caller-provided buffer from one shared heap block at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(reference, start, target.len())?;

        self.fill_extent_bytes(extent, byte_offset, target)
    }

    /// Return the trace map for one shared heap reference.
    pub fn scan(
        &self,
        reference: SharedHeapReference,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        let (extent, _) = self.resolve_range(reference, 0, 0)?;

        self.trace_map_for_place(extent.storage, trace_table)
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
        let (extent, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.publish_extent_edges(extent, byte_offset, byte_len, trace_table)
    }

    /// Publish shared edges from one already-resolved byte range.
    fn publish_extent_edges(
        &self,
        extent: SharedHeapExtent,
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
        let trace_map = self.trace_map_for_place(extent.storage, trace_table)?;
        let mut edges = Vec::new();

        let base_address = self.mapping.base_address() + extent.base.offset();
        scan_references::<SharedHeapReference>(
            &trace_map,
            ReferenceInput::mapped(base_address),
            ReferenceRange::bytes(byte_offset, byte_len),
            &mut edges,
        )?;

        // publish discovered references
        self.queue_references(None, edges)
    }

    /// Return one checked live extent and byte offset for one shared heap range.
    fn resolve_range(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedHeapExtent, usize)> {
        // resolve live block
        let Some(extent) = self.resolve_extent(reference) else {
            return Err(HeapError::invalid_shared_heap_reference(reference));
        };

        // project caller range into the block payload
        let byte_offset = block_byte_offset(extent.byte_offset, start, byte_len, extent.byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Return the trace map for one shared heap extent.
    pub(crate) fn trace_map_for_place(
        &self,
        storage: SharedHeapStorage,
        trace_table: &TraceTable,
    ) -> HeapResult<TraceMap> {
        // dispatch by physical shared heap storage
        match storage {
            SharedHeapStorage::SmallSlot(slot) => {
                self.small_slot_trace_map(slot.span_index(), slot.slot_index(), trace_table)
            }
            SharedHeapStorage::LargeBlock(block_id) => {
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

                Ok(trace_map)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared heap extent.
    fn fill_extent_bytes(
        &self,
        extent: SharedHeapExtent,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the owning storage
        match extent.storage {
            SharedHeapStorage::SmallSlot(slot) => {
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
            SharedHeapStorage::LargeBlock(block_id) => {
                // resolve the large block
                let store = self.state.read();
                let Some(block) = store.large.blocks.get(block_id.index()?).cloned() else {
                    return Err(HeapError::internal("missing large block"));
                };
                let block = block.read();

                if !block.is_live {
                    return Err(HeapError::internal("missing large block"));
                }

                self.mapping
                    .read_bytes_into(block.first_offset + byte_offset, target)
                    .map_err(HeapError::from)
            }
        }
    }
}
