use super::{RawLocation, RawPlace, RawSpace};
use crate::{HeapError, HeapResult, Payload, RawPointer};

impl RawSpace {
    /// Fill one caller-provided buffer from one raw allocation at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return one checked address for a live raw byte range.
    pub(crate) fn address(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, byte_len)?;
        let offset = location.base.offset() + byte_offset;

        self.mapping.address(offset, byte_len)
    }

    /// Return one checked mutable address for a live raw byte range.
    pub(crate) fn address_mut(
        &mut self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, byte_len)?;
        let offset = location.base.offset() + byte_offset;

        self.mapping.address(offset, byte_len)
    }

    /// Return whether one raw pointer currently refers to one live allocation.
    pub fn is_live(&self, pointer: RawPointer) -> bool {
        self.resolve_location(pointer).is_some()
    }

    /// Return the bytes for one raw allocation as one owned vector.
    pub fn read_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live allocation and requested slice
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let byte_len = checked_remaining_byte_len(location.byte_offset, location.byte_len)?;

        self.location_bytes(location, location.byte_offset, byte_len)
    }

    /// Return the bytes for one live raw location.
    fn location_bytes(
        &self,
        location: RawLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let offset = location.base.offset() + byte_offset;

        self.mapping.bytes(offset, byte_len)
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        checked_remaining_byte_len(location.byte_offset, location.byte_len)
    }

    /// Fill one caller-provided buffer from one live raw location.
    fn fill_location_bytes(
        &self,
        location: RawLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let offset = location.base.offset() + byte_offset;

        self.mapping.read(offset, target)
    }

    /// Overwrite one raw byte range.
    pub fn write_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, bytes.len())?;

        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Return one checked live location and byte offset for one raw range.
    fn checked_location_range(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(RawLocation, usize)> {
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let byte_offset =
            allocation_byte_offset(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Overwrite one byte range for one live raw location.
    fn write_location_bytes(
        &mut self,
        location: RawLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let offset = location.base.offset() + byte_offset;

        self.mapping.write(offset, bytes)
    }

    /// Overwrite one raw byte.
    pub fn write_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.write_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw allocation payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<RawPointer> {
        // resolve the live allocation first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.replace_location_bytes(location.place, location.byte_len, bytes)
    }

    /// Replace the full payload for one live raw location.
    fn replace_location_bytes(
        &mut self,
        place: RawPlace,
        previous_byte_len: usize,
        bytes: &[u8],
    ) -> HeapResult<RawPointer> {
        // replace the payload inside the owning place
        match place {
            RawPlace::Small(slot) => {
                let Some(class) = self.span(slot.span_index()).map(|span| span.class.clone())
                else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                // rewrite in place when the payload still fits
                if bytes.len() == class.byte_len {
                    let Some(span) = self.span(slot.span_index()) else {
                        return Err(HeapError::MissingSpan {
                            span_index: slot.span_index(),
                        });
                    };
                    let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                    let offset = span.first_offset + slot_offset;

                    self.mapping.write(offset, bytes)?;

                    self.usage.resize(previous_byte_len, bytes.len());

                    return self.base_pointer(RawPlace::Small(slot));
                }

                // otherwise allocate new place and retarget the pointer
                let new_location = match self.allocate_small_bytes(bytes)? {
                    Some(new_slot) => RawPlace::Small(new_slot),
                    None => {
                        let pages = self.allocate_page_run_zeroed(bytes.len())?;
                        let allocation_id = self.insert_large_allocation(bytes.len(), pages)?;
                        let Some(allocation) = self.large_allocation(allocation_id) else {
                            return Err(HeapError::MissingLargeAllocation {
                                allocation_id: allocation_id.id(),
                            });
                        };
                        let first_offset = allocation.first_offset;

                        if let Err(error) = self.mapping.write(first_offset, bytes) {
                            let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                                return Err(HeapError::MissingLargeAllocation {
                                    allocation_id: allocation_id.id(),
                                });
                            };
                            let pages = allocation.pages;

                            allocation.retire();
                            self.large
                                .free_large_allocation_ids
                                .push(allocation_id.id());
                            self.unmap_page_run(first_offset, &pages);
                            self.release_page_run(pages)?;

                            return Err(error);
                        }

                        RawPlace::Large(allocation_id)
                    }
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_location)
            }
            RawPlace::Large(allocation_id) => {
                let new_location = self.allocate_place(bytes.len(), Payload::Bytes(bytes))?;

                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };
                let first_offset = allocation.first_offset;
                let pages = allocation.pages;

                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::MissingLargeAllocation {
                        allocation_id: allocation_id.id(),
                    });
                };

                // retire the previous large allocation after replacement succeeds
                allocation.retire();
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_location)
            }
        }
    }

    /// Return one raw byte by offset without materializing the full payload.
    pub(crate) fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        let location = self.resolve_location(pointer)?;
        let byte_offset = location.byte_offset + index;

        if byte_offset >= location.byte_len {
            return None;
        }

        let offset = location.base.offset() + byte_offset;
        let mut byte = [0];

        self.mapping.read(offset, &mut byte).ok()?;

        Some(byte[0])
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

/// Return one small-slot base offset.
fn small_slot_offset(size_class: usize, slot_index: usize) -> usize {
    size_class * slot_index
}
