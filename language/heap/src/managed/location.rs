use super::{LargeEntryId, ManagedYoungId};
use crate::alloc::SpanSlot;

/// One stable managed entry location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) enum ManagedLocation {
    /// One young-space entry.
    Young(ManagedYoungId),
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in managed large space.
    Large(LargeEntryId),
}

/// One live managed reference entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct ManagedReferenceEntry {
    /// The storage location for this managed entry, if allocated.
    location: Option<ManagedLocation>,
    /// The logical byte length for this entry.
    byte_len: usize,
}

impl ManagedReferenceEntry {
    /// Return one vacant managed reference entry.
    pub(crate) const fn vacant() -> Self {
        Self {
            location: None,
            byte_len: 0,
        }
    }

    /// Create one live managed reference entry.
    pub(crate) fn new(location: ManagedLocation, byte_len: usize) -> Self {
        Self {
            location: Some(location),
            byte_len,
        }
    }

    /// Report whether this record is vacant.
    pub(crate) fn is_vacant(self) -> bool {
        self.location.is_none()
    }

    /// Return the storage location for this managed entry.
    pub(crate) const fn location(self) -> Option<ManagedLocation> {
        self.location
    }

    /// Set the storage location for this managed entry.
    pub(crate) fn set_location(&mut self, location: ManagedLocation) {
        self.location = Some(location);
    }

    /// Return the logical byte length for this entry.
    pub(crate) const fn byte_len(self) -> usize {
        self.byte_len
    }
}
