use super::{RawLocation, RawSpace, RawStorage};
use crate::allocator::{PageView, SpanSlot};
use crate::{AccountingRegion, HeapError, HeapResult, RawPointer};

impl RawSpace {
    /// Return the projected mapped-byte delta for one raw write.
    pub fn write_mapped_byte_delta(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        self.checked_location_range(pointer, start, byte_len)?;

        Ok(0)
    }

    /// Fill one caller-provided buffer from one raw entry at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, target.len())?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return whether one raw pointer currently refers to one live entry.
    pub fn is_live(&self, pointer: RawPointer) -> bool {
        self.resolve_location(pointer).is_some()
    }

    /// Return the bytes for one raw entry as one owned vector.
    pub fn read_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live entry and requested slice
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
        // read through the storage partition that owns this location
        match location.storage {
            RawStorage::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = page_view_capacity(&span.pages, self.allocator().page_bytes())?;
                let slot_offset = checked_slot_offset(
                    slot.span_index(),
                    span.class.size_class,
                    slot.slot_index(),
                )?;
                let read_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                self.allocator()
                    .bytes_to_vec_from(&span.pages, read_offset, byte_len)
            }
            RawStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.allocator()
                    .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
            }
        }
    }

    /// Return the remaining byte length for one raw entry.
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
        // read through the storage partition that owns this location
        match location.storage {
            RawStorage::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = page_view_capacity(&span.pages, self.allocator().page_bytes())?;
                let slot_offset = checked_slot_offset(
                    slot.span_index(),
                    span.class.size_class,
                    slot.slot_index(),
                )?;
                let read_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                self.allocator()
                    .fill_bytes_from(&span.pages, read_offset, target)
            }
            RawStorage::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.allocator()
                    .fill_bytes_from(&entry.pages, byte_offset, target)
            }
        }
    }

    /// Overwrite one raw byte range.
    pub fn set_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(pointer, start, bytes.len())?;

        self.set_location_bytes(location, byte_offset, bytes)
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
            checked_byte_range(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Overwrite one byte range for one live raw location.
    fn set_location_bytes(
        &mut self,
        location: RawLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // write through the storage partition that owns this location
        match location.storage {
            RawStorage::Small(slot) => {
                let allocator = self.allocator().clone();
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = page_view_capacity(&span.pages, allocator.page_bytes())?;
                let slot_offset = checked_slot_offset(
                    slot.span_index(),
                    span.class.size_class,
                    slot.slot_index(),
                )?;
                let write_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                allocator.set_bytes_unique(&span.pages, write_offset, bytes)?;

                Ok(())
            }
            RawStorage::Large(entry_id) => {
                let allocator = self.allocator().clone();
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                allocator.set_bytes_unique(&entry.pages, byte_offset, bytes)?;

                Ok(())
            }
        }
    }

    /// Overwrite one raw byte.
    pub fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.set_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw entry payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<RawPointer> {
        // resolve the live entry first
        let Some(location) = self.resolve_location(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.replace_location_bytes(location.storage, location.byte_len, bytes)
    }

    /// Replace the full payload for one live raw location.
    fn replace_location_bytes(
        &mut self,
        storage: RawStorage,
        previous_byte_len: usize,
        bytes: &[u8],
    ) -> HeapResult<RawPointer> {
        // replace the payload inside the storage partition that owns this location
        match storage {
            RawStorage::Small(slot) => {
                let Some(class) = self.span(slot.span_index()).map(|span| span.class.clone())
                else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                // rewrite in place when the payload still fits
                if bytes.len() == class.byte_len {
                    self.replace_small_location_bytes(slot, bytes)?;

                    self.usage
                        .resize(previous_byte_len, bytes.len(), AccountingRegion::Raw)?;

                    return self.base_pointer(RawStorage::Small(slot));
                }

                // otherwise allocate new storage and retarget the pointer
                let new_location = match self.allocate_small_bytes(bytes)? {
                    Some(new_slot) => RawStorage::Small(new_slot),
                    None => {
                        let pages = self.allocate_page_view_bytes(bytes)?;
                        let entry_id = self.store_large_entry(bytes.len(), pages)?;

                        RawStorage::Large(entry_id)
                    }
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot)?;
                let _new_location = new_location;

                self.usage
                    .resize(previous_byte_len, bytes.len(), AccountingRegion::Raw)?;

                self.base_pointer(new_location)
            }
            RawStorage::Large(entry_id) => {
                let next_pointer = self.replace_large_location_bytes(entry_id, bytes)?;

                self.usage
                    .resize(previous_byte_len, bytes.len(), AccountingRegion::Raw)?;

                Ok(next_pointer)
            }
        }
    }

    /// Replace one small raw entry in place inside its current span.
    fn replace_small_location_bytes(&mut self, slot: SpanSlot, bytes: &[u8]) -> HeapResult<()> {
        let allocator = self.allocator().clone();
        let slot_index = slot.slot_index();
        {
            let Some(span) = self.span_mut(slot.span_index()) else {
                return Err(HeapError::MissingSpan {
                    span_index: slot.span_index(),
                });
            };

            // rewrite the slot payload in place
            let slot_offset =
                checked_slot_offset(slot.span_index(), span.class.size_class, slot_index)?;

            allocator.set_bytes_unique(&span.pages, slot_offset, bytes)?;
        }

        Ok(())
    }

    /// Replace one large raw entry by rebuilding its page view.
    fn replace_large_location_bytes(
        &mut self,
        entry_id: super::LargeEntryId,
        bytes: &[u8],
    ) -> HeapResult<RawPointer> {
        let Some(previous_entry) = self.large_entry(entry_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };
        let previous_pages = previous_entry.pages.clone();
        let next_pages = self.allocate_page_view_bytes(bytes)?;

        let Some(entry) = self.large_entry_mut(entry_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };

        // commit the new page view before releasing the old one
        entry.pages = next_pages.clone();

        // update the recorded payload lengths
        entry.len = bytes.len();

        // release the previous page view after the replacement is committed
        self.unmap_page_view(&previous_pages)?;
        self.map_page_view(&next_pages, |logical_page_index| {
            super::RawPageOwner::Large {
                entry_id,
                logical_page_index,
            }
        })?;
        self.release_page_view(previous_pages)?;

        let base_address = self.allocator().page_view_ptr(&next_pages, 0)? as usize;

        Ok(RawPointer::new(base_address))
    }

    /// Return one raw byte by offset without materializing the full payload.
    pub(crate) fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        let location = self.resolve_location(pointer)?;
        let byte_offset = location.byte_offset.checked_add(index)?;

        if byte_offset >= location.byte_len {
            return None;
        }

        match location.storage {
            RawStorage::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_offset = checked_slot_offset(
                    slot.span_index(),
                    span.class.size_class,
                    slot.slot_index(),
                )
                .ok()?;
                let span_byte_len =
                    page_view_capacity(&span.pages, self.allocator().page_bytes()).ok()?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, span_byte_len).ok()?;

                self.allocator()
                    .byte_at(&span.pages, span_byte_len, read_offset)
            }
            RawStorage::Large(entry_id) => {
                let entry = self.large_entry(entry_id)?;

                self.allocator()
                    .byte_at(&entry.pages, location.byte_len, byte_offset)
            }
        }
    }
}

/// Return one checked entry-local byte range start.
fn checked_byte_range(
    base_offset: usize,
    start: usize,
    len: usize,
    capacity: usize,
) -> HeapResult<usize> {
    let byte_offset = base_offset
        .checked_add(start)
        .ok_or(HeapError::InvalidByteRange {
            start: base_offset,
            len: start,
            capacity,
        })?;
    let end = byte_offset
        .checked_add(len)
        .ok_or(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        })?;

    if end > capacity {
        return Err(HeapError::InvalidByteRange {
            start: byte_offset,
            len,
            capacity,
        });
    }

    Ok(byte_offset)
}

/// Return the remaining bytes after one checked entry-local offset.
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

/// Return one checked storage-local offset.
fn checked_storage_offset(
    base_offset: usize,
    byte_offset: usize,
    capacity: usize,
) -> HeapResult<usize> {
    let offset = base_offset
        .checked_add(byte_offset)
        .ok_or(HeapError::InvalidByteRange {
            start: base_offset,
            len: byte_offset,
            capacity,
        })?;

    if offset > capacity {
        return Err(HeapError::InvalidByteRange {
            start: offset,
            len: 0,
            capacity,
        });
    }

    Ok(offset)
}

/// Return one checked small-slot base offset.
fn checked_slot_offset(
    span_index: usize,
    size_class: usize,
    slot_index: usize,
) -> HeapResult<usize> {
    size_class
        .checked_mul(slot_index)
        .ok_or(HeapError::InvalidSmallSlot {
            span_index,
            slot_index,
        })
}

/// Return the mapped byte capacity for one page view.
fn page_view_capacity(page_view: &PageView, page_bytes: usize) -> HeapResult<usize> {
    page_view
        .len()
        .checked_mul(page_bytes)
        .ok_or(HeapError::InvalidByteRange {
            start: 0,
            len: page_view.len(),
            capacity: usize::MAX,
        })
}
