use destack_mir::ReferenceMap;

use super::{HeapLocation, HeapSpace, HeapStorage};
use crate::allocator::PageView;
use crate::{HeapError, HeapReference, HeapResult};

impl HeapSpace {
    /// Return the projected mapped-byte delta for one heap write.
    pub fn write_mapped_byte_delta(
        &self,
        reference: HeapReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        self.checked_location_range(reference, start, byte_len)?;

        Ok(0)
    }

    /// Fill one caller-provided buffer from one heap entry at one offset.
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

    /// Return whether one heap reference currently refers to one live entry.
    pub fn is_live(&self, reference: HeapReference) -> bool {
        self.resolve_location(reference).is_some()
    }

    #[cfg(test)]
    /// Return the live storage location for one heap reference.
    pub(crate) fn location(&self, reference: HeapReference) -> Option<HeapStorage> {
        Some(self.resolve_location(reference)?.storage)
    }

    /// Return the reference map for one heap reference.
    pub fn scan(&self, reference: HeapReference) -> HeapResult<ReferenceMap> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        self.location_reference_map(location.storage)
    }

    /// Return the remaining byte length for one heap reference.
    pub fn byte_len(&self, reference: HeapReference) -> HeapResult<usize> {
        let Some(location) = self.resolve_location(reference) else {
            return Err(HeapError::InvalidHeapReference { reference });
        };

        checked_remaining_byte_len(location.byte_offset, location.byte_len)
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
            checked_byte_range(location.byte_offset, start, byte_len, location.byte_len)?;

        Ok((location, byte_offset))
    }

    /// Record one heap write barrier for one live heap location.
    pub(crate) fn write_barrier_location(
        &mut self,
        location: HeapLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // only mature locations need remembered-write bookkeeping
        match location.storage {
            HeapStorage::Young(_) => Ok(()),
            HeapStorage::Small(slot) => self.mark_span_slot_dirty(
                slot.span_index(),
                slot.slot_index(),
                byte_offset,
                byte_len,
            ),
            HeapStorage::Large(entry_id) => {
                self.mark_large_entry_dirty(entry_id, byte_offset, byte_len)
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

        let reference_map = self.location_reference_map(location.storage)?;

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
        // write through the storage partition that owns this location
        match location.storage {
            HeapStorage::Young(young_id) => {
                let previous_pages = self.young.pages.clone();
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                let read_offset = checked_storage_offset(
                    self.young_entry_offset(entry),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;
                let allocator = self.allocator().clone();

                allocator.set_bytes(&mut self.young.pages, read_offset, bytes)?;

                if self.young.pages != previous_pages {
                    let next_pages = self.young.pages.clone();

                    self.unmap_page_view(&previous_pages)?;
                    self.map_page_view(&next_pages, |logical_page_index| {
                        super::HeapPageOwner::Young { logical_page_index }
                    })?;
                }

                Ok(())
            }
            HeapStorage::Small(slot) => {
                let allocator = self.allocator().clone();
                let (previous_pages, next_pages) = {
                    let Some(span) = self.span_mut(slot.span_index()) else {
                        return Err(HeapError::MissingSpan {
                            span_index: slot.span_index(),
                        });
                    };
                    let previous_pages = span.pages.clone();
                    let span_byte_len = page_view_capacity(&span.pages, allocator.page_bytes())?;
                    let slot_offset = checked_slot_offset(
                        slot.span_index(),
                        span.class.size_class,
                        slot.slot_index(),
                    )?;
                    let write_offset =
                        checked_storage_offset(slot_offset, byte_offset, span_byte_len)?;

                    allocator.set_bytes(&mut span.pages, write_offset, bytes)?;

                    (previous_pages, span.pages.clone())
                };

                if next_pages != previous_pages {
                    self.unmap_page_view(&previous_pages)?;
                    self.map_page_view(&next_pages, |logical_page_index| {
                        super::HeapPageOwner::Small {
                            span_index: slot.span_index(),
                            logical_page_index,
                        }
                    })?;
                }

                Ok(())
            }
            HeapStorage::Large(entry_id) => {
                let allocator = self.allocator().clone();
                let (previous_pages, next_pages) = {
                    let Some(entry) = self.large_entry_mut(entry_id) else {
                        return Err(HeapError::MissingLargeEntry {
                            entry_id: entry_id.id(),
                        });
                    };
                    let previous_pages = entry.pages.clone();

                    allocator.set_bytes(&mut entry.pages, byte_offset, bytes)?;

                    (previous_pages, entry.pages.clone())
                };

                if next_pages != previous_pages {
                    self.unmap_page_view(&previous_pages)?;
                    self.map_page_view(&next_pages, |logical_page_index| {
                        super::HeapPageOwner::Large {
                            entry_id,
                            logical_page_index,
                        }
                    })?;
                }

                Ok(())
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

    /// Return the bytes for one heap entry as one owned vector.
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
        // read through the storage partition that owns this location
        match location.storage {
            HeapStorage::Young(young_id) => {
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };
                let read_offset = checked_storage_offset(
                    self.young_entry_offset(entry),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;

                self.allocator()
                    .bytes_to_vec_from(&self.young.pages, read_offset, byte_len)
            }
            HeapStorage::Small(slot) => {
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
            HeapStorage::Large(entry_id) => {
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

    /// Fill one caller-provided buffer from one live heap location.
    pub(crate) fn fill_location_bytes(
        &self,
        location: HeapLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the storage partition that owns this location
        match location.storage {
            HeapStorage::Young(young_id) => {
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };
                let read_offset = checked_storage_offset(
                    self.young_entry_offset(entry),
                    byte_offset,
                    self.young.capacity_bytes,
                )?;

                self.allocator()
                    .fill_bytes_from(&self.young.pages, read_offset, target)
            }
            HeapStorage::Small(slot) => {
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
            HeapStorage::Large(entry_id) => {
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
