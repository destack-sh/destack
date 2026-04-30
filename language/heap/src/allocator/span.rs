use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// One stable small-allocation slot inside one span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanSlot {
    /// The containing span index.
    span_index: u32,
    /// The slot index inside the span.
    slot_index: u32,
}

impl SpanSlot {
    /// Create one span slot.
    pub fn new(span_index: usize, slot_index: usize) -> HeapResult<Self> {
        let span_index = u32::try_from(span_index).map_err(|_| HeapError::InvalidSmallSlot {
            span_index,
            slot_index,
        })?;
        let slot_index = u32::try_from(slot_index).map_err(|_| HeapError::InvalidSmallSlot {
            span_index: span_index as usize,
            slot_index,
        })?;

        Ok(Self {
            span_index,
            slot_index,
        })
    }

    /// Create one span slot from trusted packed indexes.
    pub(crate) const fn from_raw(span_index: u32, slot_index: u32) -> Self {
        Self {
            span_index,
            slot_index,
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
