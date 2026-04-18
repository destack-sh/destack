use serde::{Deserialize, Serialize};

use super::LargeEntryId;
use crate::arena::SpanSlot;

/// One stable raw entry location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum RawLocation {
    /// One small-space entry stored in one span slot.
    Small(SpanSlot),
    /// One entry stored in raw large space.
    Large(LargeEntryId),
}

/// One live raw pointer record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RawPointerEntry {
    /// The storage location for this raw entry, if allocated.
    pub(crate) location: Option<RawLocation>,
    /// The logical byte length for this raw entry.
    pub(crate) byte_len: usize,
}

impl RawPointerEntry {
    /// Return one vacant raw pointer record.
    pub(crate) const fn vacant() -> Self {
        Self {
            location: None,
            byte_len: 0,
        }
    }

    /// Create one live raw pointer entry.
    pub(crate) const fn new(location: RawLocation, byte_len: usize) -> Self {
        Self {
            location: Some(location),
            byte_len,
        }
    }

    /// Report whether this raw pointer record is vacant.
    pub(crate) const fn is_vacant(self) -> bool {
        self.location.is_none()
    }

    /// Return the storage location for this raw entry.
    pub(crate) const fn location(self) -> Option<RawLocation> {
        self.location
    }

    /// Set the storage location for this raw entry.
    pub(crate) fn set_location(&mut self, location: RawLocation) {
        self.location = Some(location);
    }

    /// Set the logical byte length for this raw entry.
    pub(crate) fn set_byte_len(&mut self, byte_len: usize) {
        self.byte_len = byte_len;
    }
}
