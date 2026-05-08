use destack_mir::ReferenceMap;

use super::{HeapLocation, HeapPlace, HeapSpace, YoungPlace};
use crate::allocator::SpanSlot;
use crate::{
    HeapError, HeapReference, HeapResult, SharedHeapReference,
    scan_shared_references_in_bytes_range, scan_shared_references_in_range,
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

    /// Return the live place for one heap reference.
    #[cfg(test)]
    pub(crate) fn place(&self, reference: HeapReference) -> Option<HeapPlace> {
        Some(self.resolve_location(reference)?.place)
    }

    /// Return the reference map for one heap reference.
    pub fn scan(&self, reference: HeapReference) -> HeapResult<ReferenceMap> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        self.reference_map_for_place(location.place)
    }

    /// Record one heap write barrier for one live heap allocation.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.record_write_barrier(reference, location, byte_offset, byte_len)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<Vec<SharedHeapReference>> {
        let (location, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.shared_write_barrier_location_bytes(location, byte_offset, bytes)
    }

    /// Return old and new shared edges for one already-resolved heap store.
    fn shared_write_barrier_location_bytes(
        &self,
        location: HeapLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<Vec<SharedHeapReference>> {
        // skip ranges that cannot contain shared references
        let reference_map = self.reference_map_for_place(location.place)?;
        if !self.overlaps_shared_roots(&reference_map, byte_offset, bytes.len()) {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();

        // overwritten references
        let base_address = self.mapping.base_address() + location.base.offset();
        scan_shared_references_in_range(
            &reference_map,
            byte_offset,
            bytes.len(),
            base_address,
            &mut edges,
        )?;

        // inserted references
        scan_shared_references_in_bytes_range(
            &reference_map,
            byte_offset,
            bytes.len(),
            bytes,
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
    ) -> HeapResult<()> {
        // local collector metadata
        self.record_local_write(location, byte_offset, byte_len)?;

        // shared collector metadata
        self.record_shared_edge_write(reference, location, byte_offset, byte_len)
    }

    /// Record local collector metadata for one live heap location.
    fn record_local_write(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.write_major_barrier(location, byte_offset, byte_len)?;

        // only mature locations need remembered-write bookkeeping
        match location.place {
            HeapPlace::Young(YoungPlace::Range { .. }) | HeapPlace::Young(YoungPlace::Slot(_)) => {
                Ok(())
            }
            HeapPlace::Small(slot) => self.mark_span_slot_dirty(
                slot.span_index(),
                slot.slot_index(),
                byte_offset,
                byte_len,
            ),
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
    ) -> HeapResult<()> {
        // inactive local-to-shared scan
        if !self.is_scanning_shared_edges {
            return Ok(());
        }

        // skip writes that cannot touch shared references
        let reference_map = self.reference_map_for_place(location.place)?;
        if !self.overlaps_shared_roots(&reference_map, byte_offset, byte_len) {
            return Ok(());
        }

        self.queue_shared_reference(reference)
    }

    /// Fill one caller-provided buffer from one live heap location.
    pub(crate) fn fill_location_bytes(
        &self,
        location: HeapLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the owning place
        match location.place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                let read_offset = allocation.first_offset + byte_offset;

                Ok(self.mapping.read_bytes_into(read_offset, target)?)
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let read_offset = self.young_run_mapping_offset(slot, byte_offset)?;

                Ok(self.mapping.read_bytes_into(read_offset, target)?)
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = slot_offset + byte_offset;

                Ok(self
                    .mapping
                    .read_bytes_into(span.first_offset + read_offset, target)?)
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                self.mapping
                    .read_bytes_into(allocation.first_offset + byte_offset, target)
                    .map_err(HeapError::from)
            }
        }
    }

    /// Overwrite one byte range inside one live heap location.
    pub(crate) fn write_location_bytes(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // write through the owning place
        match location.place {
            HeapPlace::Young(YoungPlace::Range { first_offset }) => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                let mapping_offset = allocation.first_offset + byte_offset;

                // write payload bytes
                unsafe {
                    self.mapping.write_mapped_bytes(mapping_offset, bytes);
                }

                Ok(())
            }
            HeapPlace::Young(YoungPlace::Slot(slot)) => {
                let mapping_offset = self.young_run_mapping_offset(slot, byte_offset)?;

                // write payload bytes
                unsafe {
                    self.mapping.write_mapped_bytes(mapping_offset, bytes);
                }

                Ok(())
            }
            HeapPlace::Small(slot) => {
                let mapping_offset = {
                    let Some(span) = self.span(slot.span_index()) else {
                        return Err(HeapError::MissingSpan {
                            span_index: slot.span_index(),
                        });
                    };
                    let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                    let write_offset = slot_offset + byte_offset;

                    span.first_offset + write_offset
                };

                // write payload bytes
                unsafe {
                    self.mapping.write_mapped_bytes(mapping_offset, bytes);
                }

                Ok(())
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let mapping_offset = allocation.first_offset + byte_offset;

                // write payload bytes
                unsafe {
                    self.mapping.write_mapped_bytes(mapping_offset, bytes);
                }

                Ok(())
            }
        }
    }

    /// Return the mapping offset for one fixed-size young slot.
    fn young_run_mapping_offset(&self, slot: SpanSlot, byte_offset: usize) -> HeapResult<usize> {
        let Some(run) = self.young.run(slot.span_index()) else {
            return Err(HeapError::MissingSpan {
                span_index: slot.span_index(),
            });
        };
        let slot_offset = small_slot_offset(run.size_class, slot.slot_index());
        let read_offset = slot_offset + byte_offset;

        Ok(run.first_offset + read_offset)
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

/// Return one small-slot base offset.
fn small_slot_offset(size_class: usize, slot_index: usize) -> usize {
    size_class * slot_index
}
