use serde::{Deserialize, Serialize};

use super::AllocationId;

/// One stable raw span slot location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSlot {
    /// The containing span index.
    span_index: u32,
    /// The slot index inside the span.
    slot_index: u32,
}

impl SpanSlot {
    /// Create one raw span slot location.
    pub const fn new(span_index: usize, slot_index: usize) -> Self {
        Self {
            span_index: span_index as u32,
            slot_index: slot_index as u32,
        }
    }

    /// Return the containing span index.
    pub const fn span_index(self) -> usize {
        self.span_index as usize
    }

    /// Return the slot index inside the span.
    pub const fn slot_index(self) -> usize {
        self.slot_index as usize
    }
}

/// One stable raw allocation location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RawLocation {
    /// One vacant pointer slot.
    Vacant,
    /// One small-space allocation stored in one span slot.
    Small(SpanSlot),
    /// One allocation stored in raw large space.
    Large(AllocationId),
}

/// One live raw pointer record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPointerRecord {
    /// The storage location for this raw allocation.
    pub(crate) location: RawLocation,
    /// The logical byte length for this raw allocation.
    pub(crate) byte_len: usize,
}

impl RawPointerRecord {
    /// Return one vacant raw pointer record.
    pub const fn vacant() -> Self {
        Self {
            location: RawLocation::Vacant,
            byte_len: 0,
        }
    }

    /// Report whether this raw pointer record is vacant.
    pub const fn is_vacant(self) -> bool {
        matches!(self.location, RawLocation::Vacant)
    }
}
