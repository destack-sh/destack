use destack_mir::ReferenceMap;

use super::space::{allocation_byte_offset, small_slot_offset};
use super::{SharedHeapLocation, SharedHeapPlace, SharedHeapSpace};
use crate::{HeapError, HeapResult, SharedHeapReference, scan_shared_references_in_range};

impl SharedHeapSpace {
    /// Fill one caller-provided buffer from one shared heap allocation at one offset.
    pub fn read_bytes_into(
        &self,
        reference: SharedHeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(reference, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return the reference map for one shared heap reference.
    pub fn scan(&self, reference: SharedHeapReference) -> HeapResult<ReferenceMap> {
        let (location, _) = self.resolve_range(reference, 0, 0)?;

        self.reference_map_for_place(location.place)
    }

    /// Record one shared heap write barrier before one byte store.
    pub fn write_barrier_bytes(
        &self,
        reference: SharedHeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (_, byte_offset) = self.resolve_range(reference, start, bytes.len())?;

        self.write_shared_barrier_bytes(reference, byte_offset, bytes)
    }

    /// Record one shared heap write barrier after one completed byte store.
    pub fn write_barrier(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.resolve_range(reference, start, byte_len)?;

        self.publish_location_edges(location, byte_offset, byte_len)
    }

    /// Publish shared edges from one already-resolved byte range.
    fn publish_location_edges(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
        byte_len: usize,
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
        let reference_map = self.reference_map_for_place(location.place)?;
        let mut edges = Vec::new();

        let base_address = self.mapping.base_address() + location.base.offset();
        scan_shared_references_in_range(
            &reference_map,
            byte_offset,
            byte_len,
            base_address,
            &mut edges,
        )?;

        // publish discovered references
        self.queue_references(None, edges)
    }

    /// Return one checked live location and byte offset for one shared heap range.
    fn resolve_range(
        &self,
        reference: SharedHeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(SharedHeapLocation, usize)> {
        // resolve live allocation
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidSharedHeapReference { reference });
        };

        // project caller range into the allocation payload
        let byte_offset =
            allocation_byte_offset(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Return the reference map for one shared heap location.
    pub(crate) fn reference_map_for_place(
        &self,
        place: SharedHeapPlace,
    ) -> HeapResult<ReferenceMap> {
        // dispatch by physical shared heap place
        match place {
            SharedHeapPlace::Small(slot) => {
                self.small_slot_reference_map(slot.span_index(), slot.slot_index())
            }
            SharedHeapPlace::Large(allocation_id) => {
                let reference_map = self
                    .state
                    .read()
                    .large
                    .allocations
                    .get(allocation_id.index()?)
                    .cloned()
                    .ok_or(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    })?
                    .read()
                    .reference_map
                    .clone();

                Ok(reference_map)
            }
        }
    }

    /// Fill one caller-provided buffer from one shared heap location.
    fn fill_location_bytes(
        &self,
        location: SharedHeapLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the owning place
        match location.place {
            SharedHeapPlace::Small(slot) => {
                // resolve the small span
                let store = self.state.read();
                let Some(span) = store.small.spans.get(slot.span_index()).cloned() else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = slot_offset + byte_offset;

                self.mapping.read(span.first_offset + read_offset, target)
            }
            SharedHeapPlace::Large(allocation_id) => {
                // resolve the large allocation
                let store = self.state.read();
                let Some(allocation) = store.large.allocations.get(allocation_id.index()?).cloned()
                else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let allocation = allocation.read();

                if !allocation.is_live {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                }

                self.mapping
                    .read(allocation.first_offset + byte_offset, target)
            }
        }
    }
}
