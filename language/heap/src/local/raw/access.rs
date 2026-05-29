use super::{RawPlace, RawRegion, RawSpace};
use crate::{HeapError, HeapResult, Payload, RawAllocationShape, RawPointer};

impl RawSpace {
    /// Fill one caller-provided buffer from one raw allocation at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (region, byte_offset) = self.resolve_range(pointer, start, target.len())?;

        self.fill_region_bytes(region, byte_offset, target)
    }

    /// Return one checked address for a live raw byte range.
    pub(crate) fn address(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*const u8> {
        let (region, byte_offset) = self.resolve_range(pointer, start, byte_len)?;
        let offset = region.base.offset() + byte_offset;

        Ok(self.mapping.address(offset, byte_len)? as *const u8)
    }

    /// Return one checked mutable address for a live raw byte range.
    pub(crate) fn address_mut(
        &mut self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (region, byte_offset) = self.resolve_range(pointer, start, byte_len)?;
        let offset = region.base.offset() + byte_offset;

        Ok(self.mapping.address(offset, byte_len)?)
    }

    /// Return whether one raw pointer currently refers to one live allocation.
    pub fn is_live(&self, pointer: RawPointer) -> bool {
        self.resolve_region(pointer).is_some()
    }

    /// Return the bytes for one raw allocation as one owned vector.
    pub fn read_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live allocation and requested slice
        let Some(region) = self.resolve_region(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };
        let byte_len = region.byte_len - region.byte_offset;

        self.region_bytes(region, region.byte_offset, byte_len)
    }

    /// Return the bytes for one live raw region.
    fn region_bytes(
        &self,
        region: RawRegion,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let offset = region.base.offset() + byte_offset;

        Ok(self.mapping.read_bytes(offset, byte_len)?)
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        let Some(region) = self.resolve_region(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };

        Ok(region.byte_len - region.byte_offset)
    }

    /// Fill one caller-provided buffer from one live raw region.
    fn fill_region_bytes(
        &self,
        region: RawRegion,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let offset = region.base.offset() + byte_offset;

        Ok(self.mapping.read_bytes_into(offset, target)?)
    }

    /// Overwrite one raw byte range.
    pub fn write_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (region, byte_offset) = self.resolve_range(pointer, start, bytes.len())?;

        self.write_region_bytes(region, byte_offset, bytes)
    }

    /// Return one checked live region and byte offset for one raw range.
    fn resolve_range(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(RawRegion, usize)> {
        let Some(region) = self.resolve_region(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };
        let byte_offset =
            allocation_byte_offset(region.byte_offset, start, byte_len, region.byte_len)?;

        Ok((region, byte_offset))
    }

    /// Overwrite one byte range for one live raw region.
    fn write_region_bytes(
        &mut self,
        region: RawRegion,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let offset = region.base.offset() + byte_offset;

        self.write_mapped_bytes(offset, bytes);

        Ok(())
    }

    /// Overwrite one raw byte.
    pub fn write_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.write_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw allocation payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<RawPointer> {
        // resolve the live allocation first
        let Some(region) = self.resolve_region(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };

        self.replace_region_bytes(region.place, region.byte_len, bytes)
    }

    /// Replace the full payload for one live raw region.
    fn replace_region_bytes(
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
                    return Err(HeapError::internal("missing span"));
                };

                // rewrite in place when the payload still fits
                if bytes.len() == class.byte_len {
                    let Some(span) = self.span(slot.span_index()) else {
                        return Err(HeapError::internal("missing span"));
                    };
                    let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                    let offset = span.first_offset + slot_offset;

                    self.write_mapped_bytes(offset, bytes);

                    self.usage.resize(previous_byte_len, bytes.len());

                    return self.base_pointer(RawPlace::Small(slot));
                }

                // otherwise allocate new place and retarget the pointer
                let new_region = match self.allocate_small_bytes(bytes)? {
                    Some(new_slot) => RawPlace::Small(new_slot),
                    None => {
                        let pages = self.allocate_page_run(bytes.len())?;
                        let allocation_id = self.insert_large_allocation(bytes.len(), 1, pages)?;
                        let Some(allocation) = self.large_allocation(allocation_id) else {
                            return Err(HeapError::internal("missing large allocation"));
                        };
                        let first_offset = allocation.first_offset;

                        self.write_mapped_bytes(first_offset, bytes);

                        RawPlace::Large(allocation_id)
                    }
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_region)
            }
            RawPlace::Large(allocation_id) => {
                let shape = RawAllocationShape::bytes(bytes.len());
                let new_region = self.allocate_place(shape, Payload::Bytes(bytes))?;

                let Some(allocation) = self.large_allocation(allocation_id) else {
                    return Err(HeapError::internal("missing large allocation"));
                };
                let first_offset = allocation.first_offset;
                let pages = allocation.pages;

                let Some(allocation) = self.large_allocation_mut(allocation_id) else {
                    return Err(HeapError::internal("missing large allocation"));
                };

                // retire the previous large allocation after replacement succeeds
                allocation.retire();
                self.large
                    .free_large_allocation_ids
                    .push(allocation_id.id());
                self.unmap_page_run(first_offset, &pages);
                self.release_page_run(pages)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_region)
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

/// Return one small-slot base offset.
fn small_slot_offset(size_class: usize, slot_index: usize) -> usize {
    size_class * slot_index
}
