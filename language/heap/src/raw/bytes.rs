use super::{RawLocation, RawPointerRecord, RawSpace};
use crate::value::{RawPointer, Value};

impl RawSpace {
    /// Return whether one raw pointer currently refers to one live allocation.
    pub fn is_allocated(&self, pointer: RawPointer) -> bool {
        self.pointer(pointer).is_some()
    }

    /// Return the bytes for one raw allocation.
    pub fn bytes(&self, pointer: RawPointer) -> Option<Vec<u8>> {
        // resolve the live allocation and requested slice
        let record = self.pointer(pointer)?;
        let byte_offset = pointer.byte_offset();
        let byte_len = record.byte_len.saturating_sub(byte_offset);

        self.location_bytes(record.location, byte_offset, byte_len)
    }

    /// Return the bytes for one live raw location.
    fn location_bytes(
        &self,
        location: RawLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> Option<Vec<u8>> {
        // read through the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_offset = span.size_class.saturating_mul(slot.slot_index());
                let read_offset = slot_offset.saturating_add(byte_offset);

                Some(
                    self.arena()
                        .bytes_to_vec_from(&span.pages, read_offset, byte_len),
                )
            }
            RawLocation::Large(allocation_id) => {
                let allocation = self.allocation(allocation_id)?;

                Some(
                    self.arena()
                        .bytes_to_vec_from(&allocation.pages, byte_offset, byte_len),
                )
            }
            RawLocation::Vacant => None,
        }
    }

    /// Return the remaining byte length for one raw allocation.
    pub fn byte_len(&self, pointer: RawPointer) -> Option<usize> {
        let record = self.pointer(pointer)?;

        Some(record.byte_len.saturating_sub(pointer.byte_offset()))
    }

    /// Return the values for one raw allocation.
    pub fn values(&self, pointer: RawPointer) -> Option<Vec<Value>> {
        let bytes = self.bytes(pointer)?;

        Some(
            bytes
                .chunks_exact(Value::BYTE_LEN)
                .filter_map(Value::from_byte_slice)
                .collect(),
        )
    }

    /// Overwrite one raw byte range.
    pub fn set_bytes(&mut self, pointer: RawPointer, start: usize, bytes: &[u8]) -> bool {
        // validate the write against the live allocation bounds
        let byte_offset = pointer.byte_offset().saturating_add(start);
        let Some(record) = self.pointer(pointer) else {
            return false;
        };

        if byte_offset.saturating_add(bytes.len()) > record.byte_len {
            return false;
        }

        self.set_location_bytes(record.location, byte_offset, bytes)
    }

    /// Overwrite one byte range for one live raw location.
    fn set_location_bytes(
        &mut self,
        location: RawLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> bool {
        // write through the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let arena = self.arena().clone();
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return false;
                };

                let slot_offset = span.size_class.saturating_mul(slot.slot_index());
                let write_offset = slot_offset.saturating_add(byte_offset);

                arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            RawLocation::Large(allocation_id) => {
                let arena = self.arena().clone();
                let Some(allocation) = self.allocation_mut(allocation_id) else {
                    return false;
                };

                arena.set_bytes(&mut allocation.pages, byte_offset, bytes)
            }
            RawLocation::Vacant => false,
        }
    }

    /// Overwrite one raw byte.
    pub fn set_byte(&mut self, pointer: RawPointer, index: usize, byte: u8) -> bool {
        self.set_bytes(pointer, index, &[byte])
    }

    /// Replace the entire raw allocation payload.
    pub fn replace_bytes(&mut self, pointer: RawPointer, bytes: &[u8]) -> bool {
        // resolve the live allocation first
        let Some(record) = self.pointer(pointer).copied() else {
            return false;
        };

        self.replace_location_bytes(record.location, record.byte_len, pointer, bytes)
    }

    /// Replace the full payload for one live raw location.
    fn replace_location_bytes(
        &mut self,
        location: RawLocation,
        previous_byte_len: usize,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> bool {
        // replace the payload inside the storage partition that owns this location
        match location {
            RawLocation::Small(slot) => {
                let Some(size_class) = self.span(slot.span_index()).map(|span| span.size_class)
                else {
                    return false;
                };

                // rewrite in place when the payload still fits
                if bytes.len() <= size_class {
                    if !self.replace_small_location_bytes(slot, pointer, bytes) {
                        return false;
                    }

                    self.update_allocated_bytes(previous_byte_len, bytes.len());

                    return true;
                }

                // otherwise allocate new storage and retarget the pointer
                let new_location = if let Some(new_slot) = self.allocate_small_slot(bytes) {
                    RawLocation::Small(new_slot)
                } else {
                    let pages = self.arena().allocate_bytes(bytes);
                    let allocation_id = self.allocate_allocation_slot(bytes.len(), pages);

                    RawLocation::Large(allocation_id)
                };

                // release the previous slot before retargeting the pointer
                self.release_small_slot(slot);

                // retarget the live pointer record
                if let Some(record) = self.pointer_mut(pointer) {
                    record.location = new_location;
                    record.byte_len = bytes.len();
                }

                self.update_allocated_bytes(previous_byte_len, bytes.len());

                true
            }
            RawLocation::Large(allocation_id) => {
                let Some(previous_len) =
                    self.replace_large_location_bytes(allocation_id, pointer, bytes)
                else {
                    return false;
                };

                self.update_allocated_bytes(previous_len, bytes.len());

                true
            }
            RawLocation::Vacant => false,
        }
    }

    /// Replace one small raw allocation in place inside its current span.
    fn replace_small_location_bytes(
        &mut self,
        slot: super::SpanSlot,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> bool {
        let arena = self.arena().clone();
        let Some(span) = self.span_mut(slot.span_index()) else {
            return false;
        };

        // rewrite the slot payload in place
        let slot_index = slot.slot_index();
        let slot_offset = span.size_class.saturating_mul(slot_index);

        if !arena.set_bytes(&mut span.pages, slot_offset, bytes) {
            return false;
        }

        // then update the recorded payload lengths
        span.lengths[slot_index] = bytes.len() as u16;

        if let Some(record) = self.pointer_mut(pointer) {
            record.byte_len = bytes.len();
        }

        true
    }

    /// Replace one large raw allocation by rebuilding its page map.
    fn replace_large_location_bytes(
        &mut self,
        allocation_id: super::AllocationId,
        pointer: RawPointer,
        bytes: &[u8],
    ) -> Option<usize> {
        let arena = self.arena().clone();
        let allocation = self.allocation_mut(allocation_id)?;

        // release the previous page map before rebuilding it
        let pages = allocation.pages.clone();
        arena.release_pages(&pages);
        allocation.pages = arena.allocate_bytes(bytes);

        // update the recorded payload lengths
        let previous_len = allocation.len;
        allocation.len = bytes.len();

        if let Some(record) = self.pointer_mut(pointer) {
            record.byte_len = bytes.len();
        }

        Some(previous_len)
    }

    /// Update the total raw allocation bytes after one replacement.
    fn update_allocated_bytes(&mut self, previous_len: usize, next_len: usize) {
        self.allocated_bytes = self
            .allocated_bytes
            .saturating_sub(previous_len as u64)
            .saturating_add(next_len as u64);
    }

    /// Return one live raw pointer record.
    pub(super) fn pointer(&self, pointer: RawPointer) -> Option<&RawPointerRecord> {
        // resolve the dense pointer slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let record = self.pointers.get(index)?;

        // skip vacant pointer records
        (!record.is_vacant()).then_some(record)
    }

    /// Return one live raw pointer record mutably.
    pub(super) fn pointer_mut(&mut self, pointer: RawPointer) -> Option<&mut RawPointerRecord> {
        // resolve the dense pointer slot first
        let index = pointer.id().checked_sub(1)? as usize;
        let record = self.pointers.get_mut(index)?;

        // skip vacant pointer records
        (!record.is_vacant()).then_some(record)
    }
}
