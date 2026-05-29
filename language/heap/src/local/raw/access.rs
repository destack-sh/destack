use super::{RawExtent, RawSpace, RawStorage};
use crate::{HeapError, HeapResult, Payload, RawAllocationShape, RawPointer};

impl RawSpace {
    /// Fill one caller-provided buffer from one raw block at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, target.len())?;

        self.fill_extent_bytes(extent, byte_offset, target)
    }

    /// Return one checked address for a live raw byte range.
    pub(crate) fn address(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*const u8> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, byte_len)?;
        let offset = extent.base.offset() + byte_offset;

        Ok(self.mapping.address(offset, byte_len)? as *const u8)
    }

    /// Return one checked mutable address for a live raw byte range.
    pub(crate) fn address_mut(
        &mut self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<*mut u8> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, byte_len)?;
        let offset = extent.base.offset() + byte_offset;

        Ok(self.mapping.address(offset, byte_len)?)
    }

    /// Return whether one raw pointer currently refers to one live block.
    pub fn is_live(&self, pointer: RawPointer) -> bool {
        self.resolve_extent(pointer).is_some()
    }

    /// Return the bytes for one raw block as one owned vector.
    pub fn read_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live block and requested slice
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };
        let byte_len = extent.byte_len - extent.byte_offset;

        self.extent_bytes(extent, extent.byte_offset, byte_len)
    }

    /// Return the bytes for one live raw extent.
    fn extent_bytes(
        &self,
        extent: RawExtent,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        let offset = extent.base.offset() + byte_offset;

        Ok(self.mapping.read_bytes(offset, byte_len)?)
    }

    /// Return the remaining byte length for one raw block.
    pub fn byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };

        Ok(extent.byte_len - extent.byte_offset)
    }

    /// Fill one caller-provided buffer from one live raw extent.
    fn fill_extent_bytes(
        &self,
        extent: RawExtent,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let offset = extent.base.offset() + byte_offset;

        Ok(self.mapping.read_bytes_into(offset, target)?)
    }

    /// Overwrite one raw byte range.
    pub fn write_bytes(
        &mut self,
        pointer: RawPointer,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (extent, byte_offset) = self.resolve_range(pointer, start, bytes.len())?;

        self.write_extent_bytes(extent, byte_offset, bytes)
    }

    /// Return one checked live extent and byte offset for one raw range.
    fn resolve_range(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(RawExtent, usize)> {
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };
        let byte_offset = block_byte_offset(extent.byte_offset, start, byte_len, extent.byte_len)?;

        Ok((extent, byte_offset))
    }

    /// Overwrite one byte range for one live raw extent.
    fn write_extent_bytes(
        &mut self,
        extent: RawExtent,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let offset = extent.base.offset() + byte_offset;

        self.write_mapped_bytes(offset, bytes);

        Ok(())
    }

    /// Overwrite one raw byte.
    pub fn write_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.write_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw block payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<RawPointer> {
        // resolve the live block first
        let Some(extent) = self.resolve_extent(pointer) else {
            return Err(HeapError::invalid_raw_pointer(pointer));
        };

        self.replace_extent_bytes(extent.storage, extent.byte_len, bytes)
    }

    /// Replace the full payload for one live raw extent.
    fn replace_extent_bytes(
        &mut self,
        storage: RawStorage,
        previous_byte_len: usize,
        bytes: &[u8],
    ) -> HeapResult<RawPointer> {
        // replace the payload inside the owning storage
        match storage {
            RawStorage::SmallSlot(slot) => {
                let Some(class) = self.span(slot.span_index()).map(|span| span.class.clone())
                else {
                    return Err(HeapError::internal("missing span"));
                };

                // rewrite in storage when the payload still fits
                if bytes.len() == class.byte_len {
                    let Some(span) = self.span(slot.span_index()) else {
                        return Err(HeapError::internal("missing span"));
                    };
                    let slot_offset = small_slot_offset(span.class.size_class, slot.slot_index());
                    let offset = span.first_offset + slot_offset;

                    self.write_mapped_bytes(offset, bytes);

                    self.usage.resize(previous_byte_len, bytes.len());

                    return self.base_pointer(RawStorage::SmallSlot(slot));
                }

                // otherwise allocate new storage and retarget the pointer
                let new_storage = match self.allocate_small_bytes(bytes)? {
                    Some(new_slot) => RawStorage::SmallSlot(new_slot),
                    None => {
                        let pages = self.allocate_page_span(bytes.len())?;
                        let block_id = self.insert_large_block(bytes.len(), 1, pages)?;
                        let Some(block) = self.large_block(block_id) else {
                            return Err(HeapError::internal("missing large block"));
                        };
                        let first_offset = block.first_offset;

                        self.write_mapped_bytes(first_offset, bytes);

                        RawStorage::LargeBlock(block_id)
                    }
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_storage)
            }
            RawStorage::LargeBlock(block_id) => {
                let shape = RawAllocationShape::bytes(bytes.len());
                let new_storage = self.allocate_place(shape, Payload::Bytes(bytes))?;

                let Some(block) = self.large_block(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };
                let first_offset = block.first_offset;
                let pages = block.pages;

                let Some(block) = self.large_block_mut(block_id) else {
                    return Err(HeapError::internal("missing large block"));
                };

                // retire the previous large block after replacement succeeds
                block.retire();
                self.large.free_large_block_ids.push(block_id.id());
                self.unmap_page_span(first_offset, &pages);
                self.release_page_span(pages)?;

                self.usage.resize(previous_byte_len, bytes.len());

                self.base_pointer(new_storage)
            }
        }
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
