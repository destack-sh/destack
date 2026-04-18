use serde::{Deserialize, Serialize};

use super::SharedLargeEntryId;
use crate::arena::SpanSlot;

/// One stable shared managed entry location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SharedManagedLocation {
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in shared managed large space.
    Large(SharedLargeEntryId),
}

/// One live shared managed reference entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedManagedReferenceEntry {
    /// The storage location for this shared managed entry, if allocated.
    location: Option<SharedManagedLocation>,
    /// The logical byte length for this entry.
    byte_len: usize,
}

impl SharedManagedReferenceEntry {
    /// Return one vacant shared managed reference entry.
    pub(crate) const fn vacant() -> Self {
        Self {
            location: None,
            byte_len: 0,
        }
    }

    /// Create one live shared managed reference entry.
    pub(crate) fn new(location: SharedManagedLocation, byte_len: usize) -> Self {
        Self {
            location: Some(location),
            byte_len,
        }
    }

    /// Report whether this record is vacant.
    pub(crate) fn is_vacant(self) -> bool {
        self.location.is_none()
    }

    /// Return the storage location for this shared managed entry.
    pub(crate) const fn location(self) -> Option<SharedManagedLocation> {
        self.location
    }

    /// Return the logical byte length for this entry.
    pub(crate) const fn byte_len(self) -> usize {
        self.byte_len
    }
}
