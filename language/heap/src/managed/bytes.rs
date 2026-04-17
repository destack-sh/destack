use super::{ManagedLocation, ManagedSpace, ReferenceMap};
use crate::alloc::PageView;
use crate::value::ManagedReference;
use crate::{HeapError, HeapResult, StorageLayoutId};

impl ManagedSpace {
    /// Return the projected mapped-byte delta for one managed write.
    pub fn write_mapped_delta(
        &self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        checked_byte_range(reference.byte_offset(), start, byte_len, record.byte_len())?;

        Ok(0)
    }

    /// Fill one caller-provided buffer from one managed entry at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        reference: ManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // resolve the live entry and requested slice
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        let byte_offset = checked_byte_range(
            reference.byte_offset(),
            start,
            target.len(),
            record.byte_len(),
        )?;

        self.fill_location_bytes(location, byte_offset, target)
    }

    /// Return whether one managed reference currently refers to one live entry.
    pub fn is_live(&self, reference: ManagedReference) -> bool {
        self.reference(reference).is_some()
    }

    /// Return the live storage location for one managed reference.
    pub(crate) fn location(&self, reference: ManagedReference) -> Option<ManagedLocation> {
        self.reference(reference)?.location()
    }

    /// Return the storage layout id for one managed reference.
    pub fn layout_id(&self, reference: ManagedReference) -> HeapResult<Option<StorageLayoutId>> {
        // resolve the live location first
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        self.location_layout_id(location)
    }

    /// Return the storage layout id for one live managed location.
    fn location_layout_id(&self, location: ManagedLocation) -> HeapResult<Option<StorageLayoutId>> {
        // resolve the layout source for this storage partition
        match location {
            ManagedLocation::Young(young_id) => {
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                Ok(entry.layout_id)
            }
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                Ok(span.layout_id(slot.slot_index()))
            }
            ManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                Ok(entry.layout_id)
            }
        }
    }

    /// Return the reference map for one managed reference.
    pub fn reference_map(&self, reference: ManagedReference) -> HeapResult<&ReferenceMap> {
        // resolve the location-specific trace id first
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(map_id) = self.location_map_id(location) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        // then resolve the interned map
        let Some(reference_map) = self.map_table.map(map_id) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        Ok(reference_map)
    }

    /// Set the storage layout id for one managed reference.
    pub fn set_layout_id(
        &mut self,
        reference: ManagedReference,
        layout_id: StorageLayoutId,
    ) -> HeapResult<()> {
        // resolve the live location first
        let Some(location) = self.location(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        self.set_location_layout_id(location, layout_id)
    }

    /// Set the storage layout id for one live managed location.
    fn set_location_layout_id(
        &mut self,
        location: ManagedLocation,
        layout_id: StorageLayoutId,
    ) -> HeapResult<()> {
        // update the layout source for this storage partition
        match location {
            ManagedLocation::Young(young_id) => {
                let Some(entry) = self.young_entry_mut(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                entry.layout_id = Some(layout_id);

                Ok(())
            }
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };

                span.set_layout_id(slot.slot_index(), Some(layout_id));

                Ok(())
            }
            ManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                entry.layout_id = Some(layout_id);

                Ok(())
            }
        }
    }

    /// Return the remaining byte length for one managed reference.
    pub fn byte_len(&self, reference: ManagedReference) -> HeapResult<usize> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        checked_remaining_byte_len(reference.byte_offset(), record.byte_len())
    }

    /// Overwrite one managed byte range.
    pub fn set_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // validate the write against the live entry bounds
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        let byte_offset = checked_byte_range(
            reference.byte_offset(),
            start,
            bytes.len(),
            record.byte_len(),
        )?;

        self.set_location_bytes(location, byte_offset, bytes)
    }

    /// Overwrite one byte range for one live managed location.
    fn set_location_bytes(
        &mut self,
        location: ManagedLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        // write through the storage partition that owns this location
        match location {
            ManagedLocation::Young(young_id) => {
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
                let arena = self.arena().clone();

                arena.set_bytes(&mut self.young.pages, read_offset, bytes)
            }
            ManagedLocation::Small(slot) => {
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

                arena.set_bytes(&mut span.pages, write_offset, bytes)?;

                // remember mature writes that may update managed edges
                self.mark_span_slot_dirty(
                    slot.span_index(),
                    slot.slot_index(),
                    byte_offset,
                    bytes.len(),
                )?;

                Ok(())
            }
            ManagedLocation::Large(entry_id) => {
                let arena = self.arena().clone();
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                arena.set_bytes(&mut entry.pages, byte_offset, bytes)?;

                // remember mature writes that may update managed edges
                self.mark_large_entry_dirty(entry_id, byte_offset, bytes.len())?;

                Ok(())
            }
        }
    }

    /// Overwrite one managed byte.
    pub fn set_byte(
        &mut self,
        reference: ManagedReference,
        index: usize,
        byte: u8,
    ) -> HeapResult<()> {
        self.set_bytes(reference, index, &[byte])
    }

    /// Return the bytes for one managed entry as one owned vector.
    pub fn read_bytes(&self, reference: ManagedReference) -> HeapResult<Vec<u8>> {
        // resolve the live entry and requested slice
        let byte_offset = reference.byte_offset();
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let byte_len = checked_remaining_byte_len(byte_offset, record.byte_len())?;
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        self.location_bytes(location, byte_offset, byte_len)
    }

    /// Return the bytes for one live managed location as one owned vector.
    fn location_bytes(
        &self,
        location: ManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<Vec<u8>> {
        // read through the storage partition that owns this location
        match location {
            ManagedLocation::Young(young_id) => {
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

                self.arena()
                    .bytes_to_vec_from(&self.young.pages, read_offset, byte_len)
            }
            ManagedLocation::Small(slot) => {
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
            ManagedLocation::Large(entry_id) => {
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

    /// Fill one caller-provided buffer from one live managed location.
    fn fill_location_bytes(
        &self,
        location: ManagedLocation,
        byte_offset: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        // read through the storage partition that owns this location
        match location {
            ManagedLocation::Young(young_id) => {
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

                self.arena()
                    .fill_bytes_from(&self.young.pages, read_offset, target)
            }
            ManagedLocation::Small(slot) => {
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
            ManagedLocation::Large(entry_id) => {
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
