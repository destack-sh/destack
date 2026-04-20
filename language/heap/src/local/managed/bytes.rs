use super::{HeapScan, ManagedLocation, ManagedSpace};
use crate::arena::PageView;
use crate::{HeapError, HeapResult, LayoutId, ManagedReference, Shape, ShapeId};

impl ManagedSpace {
    /// Return the projected mapped-byte delta for one managed write.
    pub fn write_mapped_delta(
        &self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<i64> {
        self.checked_location_range(reference, start, byte_len)?;

        Ok(0)
    }

    /// Fill one caller-provided buffer from one managed entry at one offset.
    pub(crate) fn read_bytes_into(
        &self,
        reference: ManagedReference,
        start: usize,
        target: &mut [u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) =
            self.checked_location_range(reference, start, target.len())?;

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
    pub fn layout_id(&self, reference: ManagedReference) -> HeapResult<Option<LayoutId>> {
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
    fn location_layout_id(&self, location: ManagedLocation) -> HeapResult<Option<LayoutId>> {
        let shape = self.location_shape(location)?;

        Ok(shape.layout_id)
    }

    /// Return the scan metadata for one managed reference.
    pub fn scan(&self, reference: ManagedReference) -> HeapResult<&HeapScan> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let shape = self.location_shape(location)?;

        Ok(&shape.scan)
    }

    /// Set the storage layout id for one managed reference.
    pub fn set_layout_id(
        &mut self,
        reference: ManagedReference,
        layout_id: LayoutId,
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
        layout_id: LayoutId,
    ) -> HeapResult<()> {
        // update the layout source for this storage partition
        match location {
            ManagedLocation::Young(young_id) => {
                let shape_id = self
                    .young_entry(young_id)
                    .ok_or(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    })?
                    .shape_id;
                let shape_id = self.shape_with_layout(shape_id, Some(layout_id))?;
                let Some(entry) = self.young_entry_mut(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };
                entry.shape_id = shape_id;

                Ok(())
            }
            ManagedLocation::Small(slot) => {
                let shape_id = self
                    .span(slot.span_index())
                    .ok_or(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    })?
                    .shape_ids
                    .get(slot.slot_index())
                    .copied()
                    .flatten()
                    .ok_or(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    })?;
                let shape_id = self.shape_with_layout(shape_id, Some(layout_id))?;
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                span.set_shape_id(slot.slot_index(), Some(shape_id));

                Ok(())
            }
            ManagedLocation::Large(entry_id) => {
                let shape_id = self
                    .large_entry(entry_id)
                    .ok_or(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    })?
                    .shape_id;
                let shape_id = self.shape_with_layout(shape_id, Some(layout_id))?;
                let Some(entry) = self.large_entry_mut(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };
                entry.shape_id = shape_id;

                Ok(())
            }
        }
    }

    /// Return one interned shape with one replacement layout id.
    fn shape_with_layout(
        &mut self,
        shape_id: ShapeId,
        layout_id: Option<LayoutId>,
    ) -> HeapResult<ShapeId> {
        let shape = self
            .shape_table
            .shape(shape_id)
            .cloned()
            .ok_or(HeapError::InvalidShapeId {
                index: shape_id.index(),
            })?;

        self.shape_table.intern(Shape {
            scan: shape.scan,
            layout_id,
        })
    }

    /// Return the entry shape for one live managed location.
    fn location_shape(&self, location: ManagedLocation) -> HeapResult<&Shape> {
        let shape_id = match location {
            ManagedLocation::Young(young_id) => {
                let Some(entry) = self.young_entry(young_id) else {
                    return Err(HeapError::MissingYoungEntry {
                        generation: young_id.generation(),
                        entry_index: young_id.index(),
                    });
                };

                entry.shape_id
            }
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span(slot.span_index()) else {
                    return Err(HeapError::MissingSpan {
                        span_index: slot.span_index(),
                    });
                };
                let Some(shape_id) = span.shape_ids.get(slot.slot_index()).copied().flatten()
                else {
                    return Err(HeapError::MissingSmallSlot {
                        span_index: slot.span_index(),
                        slot_index: slot.slot_index(),
                    });
                };

                shape_id
            }
            ManagedLocation::Large(entry_id) => {
                let Some(entry) = self.large_entry(entry_id) else {
                    return Err(HeapError::MissingLargeEntry {
                        entry_id: entry_id.id(),
                    });
                };

                entry.shape_id
            }
        };

        self.shape_table
            .shape(shape_id)
            .ok_or(HeapError::InvalidShapeId {
                index: shape_id.index(),
            })
    }

    /// Return the remaining byte length for one managed reference.
    pub fn byte_len(&self, reference: ManagedReference) -> HeapResult<usize> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        checked_remaining_byte_len(reference.byte_offset(), record.byte_len())
    }

    /// Overwrite one managed byte range.
    pub fn write_bytes(
        &mut self,
        reference: ManagedReference,
        start: usize,
        bytes: &[u8],
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, bytes.len())?;

        self.write_location_bytes(location, byte_offset, bytes)
    }

    /// Record one managed write barrier for one live managed allocation.
    pub fn write_barrier(
        &mut self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        let (location, byte_offset) = self.checked_location_range(reference, start, byte_len)?;

        self.write_barrier_location(location, byte_offset, byte_len)?;
        self.write_shared_barrier(reference, location, byte_offset, byte_len)
    }

    /// Return one checked live location and byte offset for one managed range.
    fn checked_location_range(
        &self,
        reference: ManagedReference,
        start: usize,
        byte_len: usize,
    ) -> HeapResult<(ManagedLocation, usize)> {
        let Some(record) = self.reference(reference) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let Some(location) = record.location() else {
            return Err(HeapError::InvalidManagedReference { reference });
        };
        let byte_offset =
            checked_byte_range(reference.byte_offset(), start, byte_len, record.byte_len())?;

        Ok((location, byte_offset))
    }

    /// Record one managed write barrier for one live managed location.
    fn write_barrier_location(
        &mut self,
        location: ManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        // only mature locations need remembered-write bookkeeping
        match location {
            ManagedLocation::Young(_) => Ok(()),
            ManagedLocation::Small(slot) => self.mark_span_slot_dirty(
                slot.span_index(),
                slot.slot_index(),
                byte_offset,
                byte_len,
            ),
            ManagedLocation::Large(entry_id) => {
                self.mark_large_entry_dirty(entry_id, byte_offset, byte_len)
            }
        }
    }

    /// Record one local-to-shared write barrier for one live managed location.
    fn write_shared_barrier(
        &mut self,
        reference: ManagedReference,
        location: ManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> HeapResult<()> {
        if !self.is_scanning_shared_roots {
            return Ok(());
        }

        let Some(shape_id) = self.location_shape_id(location) else {
            return Err(HeapError::InvalidManagedReference { reference });
        };

        if !self.overlaps_shared_roots(shape_id, byte_offset, byte_len)? {
            return Ok(());
        }

        self.queue_shared_reference(reference)
    }

    /// Overwrite one byte range for one live managed location.
    fn write_location_bytes(
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

                arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            ManagedLocation::Large(entry_id) => {
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

    /// Overwrite one managed byte.
    pub fn write_byte(
        &mut self,
        reference: ManagedReference,
        index: usize,
        byte: u8,
    ) -> HeapResult<()> {
        self.write_bytes(reference, index, &[byte])
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
    pub(crate) fn fill_location_bytes(
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
