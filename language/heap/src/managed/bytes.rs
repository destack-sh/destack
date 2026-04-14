use destack_mir::LayoutId;

use super::{ManagedLocation, ManagedSpace, ReferenceMap, StoredLayoutId};
use crate::value::ManagedReference;

impl ManagedSpace {
    /// Return whether one managed reference currently refers to one live allocation.
    pub fn is_allocated(&self, reference: ManagedReference) -> bool {
        self.reference(reference).is_some()
    }

    /// Return the live storage location for one managed reference.
    pub(crate) fn location(&self, reference: ManagedReference) -> Option<ManagedLocation> {
        Some(self.reference(reference)?.location())
    }

    /// Return the physical layout id for one managed reference.
    pub fn layout_id(&self, reference: ManagedReference) -> Option<LayoutId> {
        // resolve the live location first
        let record = self.reference(reference)?;
        let location = record.location();

        self.location_layout_id(location)
    }

    /// Return the physical layout id for one live managed location.
    fn location_layout_id(&self, location: ManagedLocation) -> Option<LayoutId> {
        // resolve the layout source for this storage partition
        match location {
            ManagedLocation::Young(young_id) => {
                let allocation = self.young_allocation(young_id)?;

                allocation.layout_id.to_option()
            }
            ManagedLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;

                self.span_layout_id(span, slot.slot_index())
            }
            ManagedLocation::Large(allocation_id) => {
                let allocation = self.allocation(allocation_id)?;

                allocation.layout_id.to_option()
            }
            ManagedLocation::Vacant => None,
        }
    }

    /// Return the reference map for one managed reference.
    pub fn reference_map(&self, reference: ManagedReference) -> Option<&ReferenceMap> {
        // resolve the location-specific trace id first
        let location = self.reference(reference)?.location();
        let trace_id = self.location_trace_id(location)?;

        // then resolve the interned map
        self.reference_map_table.map(trace_id)
    }

    /// Set the physical layout id for one managed reference.
    pub fn set_layout_id(&mut self, reference: ManagedReference, layout_id: LayoutId) -> bool {
        // resolve the live location first
        let Some(location) = self.location(reference) else {
            return false;
        };

        self.set_location_layout_id(location, layout_id)
    }

    /// Set the physical layout id for one live managed location.
    fn set_location_layout_id(&mut self, location: ManagedLocation, layout_id: LayoutId) -> bool {
        // update the layout source for this storage partition
        match location {
            ManagedLocation::Young(young_id) => {
                let Some(allocation) = self.young_allocation_mut(young_id) else {
                    return false;
                };

                allocation.layout_id = StoredLayoutId::from_option(Some(layout_id));

                true
            }
            ManagedLocation::Small(slot) => {
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return false;
                };

                Self::set_span_layout_id(span, slot.slot_index(), Some(layout_id));

                true
            }
            ManagedLocation::Large(allocation_id) => {
                let Some(allocation) = self.allocation_mut(allocation_id) else {
                    return false;
                };

                allocation.layout_id = StoredLayoutId::from_option(Some(layout_id));

                true
            }
            ManagedLocation::Vacant => false,
        }
    }

    /// Return the nominal type id for one managed reference.
    pub fn type_id(&self, reference: ManagedReference) -> Option<u32> {
        self.reference(reference)?.type_id()
    }

    /// Return the remaining byte length for one managed reference.
    pub fn byte_len(&self, reference: ManagedReference) -> Option<usize> {
        let record = self.reference(reference)?;

        Some(record.byte_len().saturating_sub(reference.byte_offset()))
    }

    /// Set the nominal type id for one managed reference.
    pub fn set_type_id(&mut self, reference: ManagedReference, type_id: u32) -> bool {
        let Some(record) = self.reference_mut(reference) else {
            return false;
        };

        record.set_type_id(Some(type_id));

        true
    }

    /// Overwrite one managed byte range.
    pub fn set_bytes(&mut self, reference: ManagedReference, start: usize, bytes: &[u8]) -> bool {
        // validate the write against the live allocation bounds
        let byte_offset = reference.byte_offset().saturating_add(start);
        let Some(record) = self.reference(reference) else {
            return false;
        };
        let location = record.location();

        if byte_offset.saturating_add(bytes.len()) > record.byte_len() {
            return false;
        }

        self.set_location_bytes(location, byte_offset, bytes)
    }

    /// Overwrite one byte range for one live managed location.
    fn set_location_bytes(
        &mut self,
        location: ManagedLocation,
        byte_offset: usize,
        bytes: &[u8],
    ) -> bool {
        // write through the storage partition that owns this location
        match location {
            ManagedLocation::Young(young_id) => {
                let Some(allocation) = self.young_allocation(young_id) else {
                    return false;
                };

                let read_offset = self
                    .young_allocation_offset(allocation)
                    .saturating_add(byte_offset);
                let arena = self.arena().clone();

                arena.set_bytes(&mut self.young.pages, read_offset, bytes)
            }
            ManagedLocation::Small(slot) => {
                let arena = self.arena().clone();
                let Some(span) = self.span_mut(slot.span_index()) else {
                    return false;
                };

                let slot_offset = span.size_class.saturating_mul(slot.slot_index());
                let write_offset = slot_offset.saturating_add(byte_offset);

                arena.set_bytes(&mut span.pages, write_offset, bytes)
            }
            ManagedLocation::Large(allocation_id) => {
                let arena = self.arena().clone();
                let Some(allocation) = self.allocation_mut(allocation_id) else {
                    return false;
                };

                arena.set_bytes(&mut allocation.pages, byte_offset, bytes)
            }
            ManagedLocation::Vacant => false,
        }
    }

    /// Overwrite one managed byte.
    pub fn set_byte(&mut self, reference: ManagedReference, index: usize, byte: u8) -> bool {
        self.set_bytes(reference, index, &[byte])
    }

    /// Return the bytes for one managed allocation as one owned vector.
    pub fn bytes(&self, reference: ManagedReference) -> Option<Vec<u8>> {
        // resolve the live allocation and requested slice
        let byte_offset = reference.byte_offset();
        let record = self.reference(reference)?;
        let byte_len = record.byte_len().saturating_sub(byte_offset);
        let location = record.location();

        self.location_bytes(location, byte_offset, byte_len)
    }

    /// Return the bytes for one live managed location as one owned vector.
    fn location_bytes(
        &self,
        location: ManagedLocation,
        byte_offset: usize,
        byte_len: usize,
    ) -> Option<Vec<u8>> {
        // read through the storage partition that owns this location
        match location {
            ManagedLocation::Young(young_id) => {
                let allocation = self.young_allocation(young_id)?;
                let read_offset = self
                    .young_allocation_offset(allocation)
                    .saturating_add(byte_offset);

                Some(
                    self.arena()
                        .bytes_to_vec_from(&self.young.pages, read_offset, byte_len),
                )
            }
            ManagedLocation::Small(slot) => {
                let span = self.span(slot.span_index())?;
                let slot_offset = span.size_class.saturating_mul(slot.slot_index());
                let read_offset = slot_offset.saturating_add(byte_offset);

                Some(
                    self.arena()
                        .bytes_to_vec_from(&span.pages, read_offset, byte_len),
                )
            }
            ManagedLocation::Large(allocation_id) => {
                let allocation = self.allocation(allocation_id)?;

                Some(
                    self.arena()
                        .bytes_to_vec_from(&allocation.pages, byte_offset, byte_len),
                )
            }
            ManagedLocation::Vacant => None,
        }
    }
}
