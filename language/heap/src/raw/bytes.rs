use super::{RawLocation, RawPointerEntry, RawSpace};
use crate::alloc::{PageView, SpanSlot};
use crate::value::{RawPointer, Value};
use crate::{HeapError, HeapResult};

impl RawSpace {
    /// Return the projected mapped-byte delta for one raw write.
    pub fn write_mapped_delta(
        &self,
        pointer: RawPointer,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        let Some(record) = self.pointer(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        checked_byte_range(pointer.byte_offset(), start, byte_len, record.byte_len)?;

        Ok(0)
    }

    /// Fill one caller-provided buffer from one raw entry at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        pointer: RawPointer,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // resolve the live entry and requested slice
        let Some(record) = self.pointer(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let byte_offset =
            checked_byte_range(pointer.byte_offset(), start, target.len(), record.byte_len)?;

        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return whether one raw pointer currently refers to one live entry.
    pub fn is_live(&self, pointer: RawPointer) -> bool {
        self.pointer(pointer).is_some()
    }

    /// Return the bytes for one raw entry as one owned vector.
    pub fn read_bytes(&self, pointer: RawPointer) -> HeapResult<Vec<u8>> {
        // resolve the live entry and requested slice
        let Some(record) = self.pointer(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let byte_offset = pointer.byte_offset();
        let byte_len = checked_remaining_byte_len(byte_offset, record.byte_len)?;
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.location_bytes(location, byte_offset, byte_len)
    }

    /// Return the bytes for one live raw location.
    fn location_bytes(
        &self,
        location: RawLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        // read through the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = arena_page_capacity(&span.pages, self.arena().page_bytes())?;
                let slot_offset =
                    checked_slot_offset(slot.span_index(), span.size_class, slot.slot_index())?;
                let read_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                self.arena()
                    .bytes_to_vec_from(&span.pages, read_offset, byte_len)
            }
            RawLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.arena()
                    .bytes_to_vec_from(&entry.pages, byte_offset, byte_len)
            }
        }
    }

    /// Return the remaining byte length for one raw entry.
    pub fn byte_len(&self, pointer: RawPointer) -> HeapResult<usize> {
        let Some(record) = self.pointer(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        checked_remaining_byte_len(pointer.byte_offset(), record.byte_len)
    }

    /// Return the values for one raw entry.
    pub fn values(&self, pointer: RawPointer) -> HeapResult<Vec<Value>> {
        let bytes = self.read_bytes(pointer)?;
        let mut values = Vec::new();

        // decode each full value slot from the materialized payload
        for bytes in bytes.chunks_exact(Value::BYTE_LEN) {
            if let Some(value) = Value::from_byte_slice(bytes) {
                values.push(value);
            }
        }

        Ok(values)
    }

    /// Fill one caller-provided buffer from one live raw location.
    fn fill_location_bytes(
        &self,
        location: RawLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let span_byte_len = arena_page_capacity(&span.pages, self.arena().page_bytes())?;
                let slot_offset =
                    checked_slot_offset(slot.span_index(), span.size_class, slot.slot_index())?;
                let read_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                self.arena()
                    .fill_bytes_from(&span.pages, read_offset, target)
            }
            RawLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                self.arena()
                    .fill_bytes_from(&entry.pages, byte_offset, target)
            }
        }
    }

    /// Overwrite one raw byte range.
    pub fn set_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> HeapResult<()> {
        // validate the write against the live entry bounds
        let Some(record) = self.pointer(pointer) else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };
        let byte_offset =
            checked_byte_range(pointer.byte_offset(), start, bytes.len(), record.byte_len)?;

        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.set_location_bytes(location, byte_offset, bytes)
    }

    /// Overwrite one byte range for one live raw location.
    fn set_location_bytes(
        &mut self,
        location: RawLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // write through the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let arena = self.arena().clone();
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                let span_byte_len = arena_page_capacity(&span.pages, arena.page_bytes())?;
                let slot_offset =
                    checked_slot_offset(slot.span_index(), span.size_class, slot.slot_index())?;
                let write_offset = checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            RawLocation::Large(entry_id) => {
                let arena = self.arena().clone();
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                arena.set_bytes(&mut entry.pages, byte_offset, bytes)
            }
        }
    }

    /// Overwrite one raw byte.
    pub fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> HeapResult<()> {
        self.set_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw entry payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> HeapResult<()> {
        // resolve the live entry first
        let Some(record) = self.pointer(pointer).copied() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        let Some(location) = record.location() else {
            return Err(HeapError::InvalidRawPointer { pointer });
        };

        self.replace_location_bytes(location, record.byte_len, pointer, bytes)
    }

    /// Replace the full payload for one live raw location.
    fn replace_location_bytes(
        &mut self,
        location: RawLocation,
        previous_byte_len: usize,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // replace the payload inside the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let Some(size_class) = self.span(slot.span_index()).map(|span| span.size_class)
                else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                // rewrite in place when the payload still fits
                if bytes.len() <= size_class {
                    self.replace_small_location_bytes(slot, pointer, bytes)?;

                    self.totals
                        .resize(previous_byte_len, bytes.len(), crate::HeapDomain::Raw)?;

                    return Ok(());
                }

                // otherwise allocate new storage and retarget the pointer
                let new_location = match self.allocate_small_bytes(bytes)? {
                    Some(new_slot) => RawLocation::Small(new_slot),
                    None => {
                        let pages = self.arena().allocate_bytes(bytes)?;
                        let entry_id = self.store_large_entry(bytes.len(), pages)?;

                        RawLocation::Large(entry_id)
                    }
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot)?;

                // retarget the live pointer record
                if let Some(record) = self.pointer_mut(pointer) {
                    record.set_location(new_location);
                    record.set_byte_len(bytes.len());
                }

                self.totals
                    .resize(previous_byte_len, bytes.len(), crate::HeapDomain::Raw)?;

                Ok(())
            }
            RawLocation::Large(entry_id) => {
                let previous_len = self.replace_large_location_bytes(entry_id, pointer, bytes)?;

                self.totals
                    .resize(previous_len, bytes.len(), crate::HeapDomain::Raw)?;

                Ok(())
            }
        }
    }

    /// Replace one small raw entry in place inside its current span.
    fn replace_small_location_bytes(
        &mut self,
        slot: SpanSlot,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let arena = self.arena().clone();
        let Some(span) = self.span_mut(slot.span_index()) else {
            return Err(HeapError::MissingSpan {
                span_index: slot.span_index(),
            });
        };

        // rewrite the slot payload in place
        let slot_index = slot.slot_index();
        let slot_offset = checked_slot_offset(slot.span_index(), span.size_class, slot_index)?;

        arena.set_bytes(&mut span.pages, slot_offset, bytes)?;

        // then update the recorded payload lengths
        span.lengths[slot_index] = bytes.len();

        if let Some(record) = self.pointer_mut(pointer) {
            record.set_byte_len(bytes.len());
        }

        Ok(())
    }

    /// Replace one large raw entry by rebuilding its page view.
    fn replace_large_location_bytes(
        &mut self,
        entry_id: super::LargeEntryId,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> HeapResult<usize> {
        let arena = self.arena().clone();
        let Some(previous_entry) = self.large_entry(entry_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };
        let previous_pages = previous_entry.pages;
        let next_pages = arena.allocate_bytes(bytes)?;

        let Some(entry) = self.large_entry_mut(entry_id) else {
            return Err(HeapError::MissingLargeEntry {
                entry_id: entry_id.id(),
            });
        };

        // commit the new page view before releasing the old one
        entry.pages = next_pages;

        // update the recorded payload lengths
        let previous_len = entry.len;
        entry.len = bytes.len();

        if let Some(record) = self.pointer_mut(pointer) {
            record.set_byte_len(bytes.len());
        }

        // release the previous page view after the replacement is committed
        arena.release_page_view(&previous_pages)?;

        Ok(previous_len)
    }

    /// Return one live raw pointer record.
    pub(super) fn pointer(&self, pointer: RawPointer) -> Option<&RawPointerEntry> {
        // resolve the dense pointer slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let record = self.pointers.get(index)?;

        // skip vacant pointer records
        record.location().map(|_| record)
    }

    /// Return one live raw pointer record mutably.
    pub(super) fn pointer_mut(&mut self, pointer: RawPointer) -> Option<&mut RawPointerEntry> {
        // resolve the dense pointer slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let record = self.pointers.get_mut(index)?;

        // skip vacant pointer records
        record.location().map(|_| record)
    }

    /// Return one raw byte by offset without materializing the full payload.
    pub(crate) fn byte_at(&self, pointer: RawPointer, index: usize) -> Option<u8> {
        let record = self.pointer(pointer)?;
        let byte_offset = pointer.byte_offset().checked_add(index)?;

        if byte_offset >= record.byte_len {
            return None;
        }

        match record.location()? {
            RawLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_offset =
                    checked_slot_offset(slot.span_index(), span.size_class, slot.slot_index())
                        .ok()?;
                let span_byte_len =
                    arena_page_capacity(&span.pages, self.arena().page_bytes()).ok()?;
                let read_offset =
                    checked_storage_offset(slot_offset, byte_offset, span_byte_len).ok()?;

                self.arena()
                    .byte_at(&span.pages, span_byte_len, read_offset)
            }
            RawLocation::Large(entry_id) => {
                let entry = self.large_entry(entry_id)?;

                self.arena()
                    .byte_at(&entry.pages, record.byte_len, byte_offset)
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
fn arena_page_capacity(page_view: &PageView, page_bytes: usize) -> HeapResult<usize> {
    page_view
        .len()
        .checked_mul(page_bytes)
        .ok_or(HeapError::InvalidByteRange {
            start: 0,
            len: page_view.len(),
            capacity: usize::MAX,
        })
}
