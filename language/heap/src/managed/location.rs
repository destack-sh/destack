use super::{AllocationId, ManagedYoungId};
use std::process::abort;

/// The stored location kind for one packed managed location.
const MANAGED_LOCATION_VACANT: u32 = 0;

/// The stored location kind for one young managed allocation.
const MANAGED_LOCATION_YOUNG: u32 = 1;

/// The stored location kind for one small managed allocation.
const MANAGED_LOCATION_SMALL: u32 = 2;

/// The stored location kind for one large managed allocation.
const MANAGED_LOCATION_LARGE: u32 = 3;

/// The bit shift for the packed managed-location kind.
const MANAGED_LOCATION_KIND_SHIFT: u32 = 30;

/// The bit mask for the packed managed-location kind.
const MANAGED_LOCATION_KIND_MASK: u32 = 0b11 << MANAGED_LOCATION_KIND_SHIFT;

/// The bit mask for the packed managed-location payload.
const MANAGED_LOCATION_VALUE_MASK: u32 = !MANAGED_LOCATION_KIND_MASK;

/// The sentinel for one absent nominal type id.
const EMPTY_MANAGED_TYPE_ID: u32 = u32::MAX;

/// One stable span slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct SpanSlot {
    /// The containing span index.
    span_index: u32,
    /// The slot index inside the span.
    slot_index: u32,
}

impl SpanSlot {
    /// Create one managed span slot.
    pub(crate) const fn new(span_index: usize, slot_index: usize) -> Self {
        Self {
            span_index: span_index as u32,
            slot_index: slot_index as u32,
        }
    }

    /// Return the containing span index.
    pub(crate) const fn span_index(self) -> usize {
        self.span_index as usize
    }

    /// Return the slot index inside the span.
    pub(crate) const fn slot_index(self) -> usize {
        self.slot_index as usize
    }
}

/// One packed managed allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub(crate) struct PackedManagedLocation {
    /// The primary packed location payload.
    primary: u32,
    /// The secondary packed location payload plus kind bits.
    secondary: u32,
}

impl PackedManagedLocation {
    /// Return one vacant managed location.
    pub(crate) const fn vacant() -> Self {
        Self {
            primary: 0,
            secondary: 0,
        }
    }

    /// Encode one managed location into its compact representation.
    pub(crate) fn encode(location: ManagedLocation) -> Self {
        match location {
            ManagedLocation::Vacant => Self::vacant(),
            ManagedLocation::Young(young_id) => Self {
                primary: young_id.generation(),
                secondary: pack_managed_location_secondary(
                    MANAGED_LOCATION_YOUNG,
                    young_id.index(),
                ),
            },
            ManagedLocation::Small(slot) => Self {
                primary: slot.span_index as u32,
                secondary: pack_managed_location_secondary(
                    MANAGED_LOCATION_SMALL,
                    slot.slot_index as u32,
                ),
            },
            ManagedLocation::Large(allocation_id) => Self {
                primary: narrow_u64(allocation_id.id()),
                secondary: pack_managed_location_secondary(MANAGED_LOCATION_LARGE, 0),
            },
        }
    }

    /// Decode this packed location.
    pub(crate) fn decode(self) -> ManagedLocation {
        let kind = self.secondary >> MANAGED_LOCATION_KIND_SHIFT;
        let value = self.secondary & MANAGED_LOCATION_VALUE_MASK;

        match kind {
            MANAGED_LOCATION_VACANT => ManagedLocation::Vacant,
            MANAGED_LOCATION_YOUNG => {
                ManagedLocation::Young(ManagedYoungId::new(self.primary, value))
            }
            MANAGED_LOCATION_SMALL => ManagedLocation::Small(SpanSlot {
                span_index: self.primary,
                slot_index: value,
            }),
            MANAGED_LOCATION_LARGE => {
                ManagedLocation::Large(AllocationId::new(self.primary as u64))
            }
            _ => fail_invalid_packed_managed_location(kind),
        }
    }
}

/// One stable managed allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManagedLocation {
    /// One vacant reference slot.
    Vacant,
    /// One young-space allocation.
    Young(ManagedYoungId),
    /// One small-space allocation stored in one span slot.
    Small(SpanSlot),
    /// One allocation stored in managed large space.
    Large(AllocationId),
}

/// One live managed reference record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ManagedReferenceRecord {
    /// The storage location for this managed allocation.
    location: PackedManagedLocation,
    /// The logical byte length for this allocation.
    byte_len: u32,
    /// The nominal type id for this allocation, if any.
    type_id: u32,
}

impl ManagedReferenceRecord {
    /// Return one vacant managed reference record.
    pub(crate) const fn vacant() -> Self {
        Self {
            location: PackedManagedLocation::vacant(),
            byte_len: 0,
            type_id: EMPTY_MANAGED_TYPE_ID,
        }
    }

    /// Create one live managed reference record.
    pub(crate) fn new(location: ManagedLocation, byte_len: usize, type_id: Option<u32>) -> Self {
        Self {
            location: PackedManagedLocation::encode(location),
            byte_len: narrow_usize(byte_len),
            type_id: type_id.unwrap_or(EMPTY_MANAGED_TYPE_ID),
        }
    }

    /// Report whether this record is vacant.
    pub(crate) fn is_vacant(self) -> bool {
        matches!(self.location(), ManagedLocation::Vacant)
    }

    /// Return the storage location for this managed allocation.
    pub(crate) fn location(self) -> ManagedLocation {
        self.location.decode()
    }

    /// Set the storage location for this managed allocation.
    pub(crate) fn set_location(&mut self, location: ManagedLocation) {
        self.location = PackedManagedLocation::encode(location);
    }

    /// Return the logical byte length for this allocation.
    pub(crate) const fn byte_len(self) -> usize {
        self.byte_len as usize
    }

    /// Return the nominal type id for this allocation.
    pub(crate) const fn type_id(self) -> Option<u32> {
        if self.type_id == EMPTY_MANAGED_TYPE_ID {
            None
        } else {
            Some(self.type_id)
        }
    }

    /// Set the nominal type id for this allocation.
    pub(crate) fn set_type_id(&mut self, type_id: Option<u32>) {
        self.type_id = type_id.unwrap_or(EMPTY_MANAGED_TYPE_ID);
    }
}

/// Pack one managed-location kind and payload into the secondary word.
fn pack_managed_location_secondary(kind: u32, value: u32) -> u32 {
    assert!(kind <= MANAGED_LOCATION_LARGE);
    assert_eq!(value & MANAGED_LOCATION_KIND_MASK, 0);

    (kind << MANAGED_LOCATION_KIND_SHIFT) | value
}

/// Narrow one `usize` to the heap's actual record width.
fn narrow_usize(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(value) => value,
        Err(_) => fail_managed_record_width_overflow(value as u128),
    }
}

/// Narrow one `u64` identifier to the packed managed-location width.
fn narrow_u64(value: u64) -> u32 {
    let value = match u32::try_from(value) {
        Ok(value) => value,
        Err(_) => fail_managed_record_width_overflow(value as u128),
    };

    assert_eq!(value & MANAGED_LOCATION_KIND_MASK, 0);

    value
}

/// Abort on one invalid packed managed-location kind.
#[cold]
fn fail_invalid_packed_managed_location(_kind: u32) -> ! {
    abort()
}

/// Abort when one managed record field exceeds the packed width.
#[cold]
fn fail_managed_record_width_overflow(_value: u128) -> ! {
    abort()
}
