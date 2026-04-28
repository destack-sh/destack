use destack_mir::ReferenceMap;

use super::{HeapLocation, HeapPlace, HeapSpace};
use crate::{
    HeapError, HeapReference, HeapResult, SharedHeapReference,
    visit_shared_references_in_reader_range,
};

impl HeapSpace {
    /// Fill one caller-provided buffer from one heap allocation at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        reference: HeapReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) =
            self.checked_location_range(reference, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
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

        self.place_reference_map(location.place)
    }

    /// Return the remaining byte length for one heap reference.
    pub fn byte_len(&self, reference: HeapReference) -> HeapResult<usize> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        checked_remaining_byte_len(location.byte_offset, location.byte_len)
    }

    /// Return one checked address for a live heap byte range.
    pub(crate) fn address(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        let mapping_offset = self.location_mapping_offset(location, byte_offset)?;

        self.mapping.address(mapping_offset, byte_len)
    }

    /// Return one checked mutable address for a live heap byte range.
    pub(crate) fn address_mut(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;
        let mapping_offset = self.location_mapping_offset(location, byte_offset)?;

        self.mapping.address(mapping_offset, byte_len)
    }

    /// Overwrite one heap byte range.
    pub fn write_bytes(
        &mut self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;

        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Record one heap write barrier for one live heap allocation.
    pub fn write_barrier(
        &mut self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;

        self.write_barrier_location(location, byte_offset, byte_len)?;
        self.write_shared_barrier(reference, location, byte_offset, byte_len)
    }

    /// Return old and new shared edges for one heap store before it writes.
    pub fn shared_write_barrier_bytes(
        &self,
        reference: HeapReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<Vec<SharedHeapReference>> {
        let (location, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;
        let reference_map = self.place_reference_map(location.place)?;
        if !self.overlaps_shared_roots(&reference_map, byte_offset, bytes.len())? {
            return Ok(Vec::new());
        }

        let mut edges = Vec::new();

        // overwritten edges
        visit_shared_references_in_reader_range(
            &reference_map,
            byte_offset,
            bytes.len(),
            |start, buffer| self.fill_location_bytes(location, start, buffer),
            |reference| {
                if !reference.is_null() {
                    edges.push(reference);
                }
            },
        )?;

        // inserted edges
        visit_shared_references_in_reader_range(
            &reference_map,
            byte_offset,
            bytes.len(),
            |start, buffer| {
                let local_start = start - byte_offset;
                let local_end = local_start + buffer.len();
                let Some(window) = bytes.get(local_start..local_end) else {
                    return Err(HeapError::TruncatedReferenceReaderWindow {
                        start: local_start,
                        width: buffer.len(),
                    });
                };

                buffer.copy_from_slice(window);
                Ok(())
            },
            |reference| {
                if !reference.is_null() {
                    edges.push(reference);
                }
            },
        )?;

        Ok(edges)
    }

    /// Return one checked live location and byte offset for one heap range.
    fn checked_location_range(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(HeapLocation, usize)> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let byte_offset =
            allocation_byte_offset(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Record one heap write barrier for one live heap location.
    pub(crate) fn write_barrier_location(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        self.write_major_barrier(location, byte_offset, byte_len)?;

        // only mature locations need remembered-write bookkeeping
        match location.place {
            HeapPlace::Young { .. } => Ok(()),
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

    /// Record one local-to-shared write barrier for one live heap location.
    fn write_shared_barrier(
        &mut self,
        reference: HeapReference,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        if !self.is_scanning_shared_edges {
            return Ok(());
        }

        let reference_map = self.place_reference_map(location.place)?;

        if !self.overlaps_shared_roots(&reference_map, byte_offset, byte_len)? {
            return Ok(());
        }

        self.queue_shared_reference(reference)
    }

    /// Overwrite one byte range for one live heap location.
    pub(crate) fn write_location_bytes(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // write through the owning place
        match location.place {
            HeapPlace::Young { first_offset } => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                let read_offset = checked_place_offset(
                    self.young_range_offset(allocation),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;

                self.mapping.write(read_offset, bytes)?;

                Ok(())
            }
            HeapPlace::Small(slot) => {
                let page_bytes = self.allocator().page_bytes();
                let mapping_offset = {
                    let Some(span) = self.span_mut(slot.span_index()) else {
                        return Err(HeapError::MissingSpan {
                            span_index: slot.span_index(),
                        });
                    };
                    let span_byte_len = span.pages.len() * page_bytes;
                    let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                    let write_offset =
                        checked_place_offset(slot_offset, byte_offset, span_byte_len)?;

                    span.first_offset + write_offset
                };

                self.mapping.write(mapping_offset, bytes)?;

                Ok(())
            }
            HeapPlace::Large(allocation_id) => {
                let mapping_offset = {
                    let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                        return Err(HeapError::MissingLargeAllocation {
                            allocation_id: allocation_id.id(),
                        });
                    };

                    allocation.first_offset + byte_offset
                };

                self.mapping.write(mapping_offset, bytes)?;

                Ok(())
            }
        }
    }

    /// Return the mapping offset for one live heap location.
    fn location_mapping_offset(
        &self,
        location: HeapLocation,
        byte_offset: usize,
    ) -> HeapResult<usize> {
        match location.place {
            HeapPlace::Young { first_offset } => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };

                checked_place_offset(
                    self.young_range_offset(allocation),
                    byte_offset,
                    self.young.capacity_bytes,
                )
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = span.pages.len() * self.allocator().page_bytes();
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_byte_len)?;

                Ok(span.first_offset + read_offset)
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                Ok(allocation.first_offset + byte_offset)
            }
        }
    }

    /// Overwrite one heap byte.
    pub fn write_byte(
        &mut self,
        reference: HeapReference,
        index: usize,
        byte: u8,
    ) -> HeapResult<()> {
        self.write_bytes(reference, index, &[byte])
    }

    /// Return the bytes for one heap allocation as one owned vector.
    pub fn read_bytes(&self, reference: HeapReference) -> HeapResult<Vec<u8>> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };
        let byte_len = checked_remaining_byte_len(location.byte_offset, location.byte_len)?;

        self.location_bytes(location, location.byte_offset, byte_len)
    }

    /// Return the bytes for one live heap location as one owned vector.
    fn location_bytes(
        &self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        // read through the owning place
        match location.place {
            HeapPlace::Young { first_offset } => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                let read_offset = checked_place_offset(
                    self.young_range_offset(allocation),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;

                self.mapping.bytes(read_offset, byte_len)
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = span.pages.len() * self.allocator().page_bytes();
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_byte_len)?;

                self.mapping
                    .bytes(span.first_offset + read_offset, byte_len)
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                self.mapping
                    .bytes(allocation.first_offset + byte_offset, byte_len)
            }
        }
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
            HeapPlace::Young { first_offset } => {
                let Some((_allocation_index, allocation)) =
                    self.young_range_by_offset(first_offset)
                else {
                    return Err(HeapError::MissingYoungRange { first_offset });
                };
                let read_offset = checked_place_offset(
                    self.young_range_offset(allocation),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;

                self.mapping.read(read_offset, target)
            }
            HeapPlace::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = span.pages.len() * self.allocator().page_bytes();
                let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                let read_offset = checked_place_offset(slot_offset, byte_offset, span_byte_len)?;

                self.mapping.read(span.first_offset + read_offset, target)
            }
            HeapPlace::Large(allocation_id) => {
                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                self.mapping
                    .read(allocation.first_offset + byte_offset, target)
            }
        }
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

    let remaining = capacity - base_offset;
    if start > remaining {
        return Err(HeapError::InvalidByteRange {
            start,
            len,
            capacity,
        });
    }

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

/// Return the remaining bytes after one checked allocation-local offset.
fn checked_remaining_byte_len(byte_offset: usize, capacity: usize) -> HeapResult<usize> {
    if byte_offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len: 0,
            capacity,
        });
    }

    Ok(capacity - byte_offset)
}

/// Return one checked place-local offset.
fn checked_place_offset(
    base_offset: usize,
    byte_offset: usize,
    capacity: usize,
) -> HeapResult<usize> {
    if byte_offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len: 0,
            capacity,
        });
    }

    let offset = base_offset + byte_offset;

    Ok(offset)
}

/// Return one small-slot base offset.
fn small_slot_offset(size_class: usize, slot_index: usize) -> usize {
    size_class * slot_index
}
