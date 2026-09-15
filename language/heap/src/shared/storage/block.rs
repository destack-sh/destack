use std::sync::Arc;

use destack_memory::MemoryRange;
use destack_mir::TraceMap;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{DropPlan, HeapError, HeapRepresentationError, HeapResult};

/// One live shared heap large block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeBlock {
    /// The first byte offset inside shared heap storage.
    pub(crate) first_offset: usize,
    /// The logical byte length of this block.
    pub(crate) byte_len: usize,
    /// The memory pages for this block.
    pub(crate) pages: MemoryRange,
    /// The trace map for this block.
    pub(crate) trace_map: Arc<TraceMap>,
    /// The drop plan for this managed block.
    pub(crate) drop: Option<DropPlan>,
    /// Whether a borrow or heap storage retained this block for the collector.
    pub(crate) retained: bool,
    /// Whether this block's values moved out, freed by the collector without its drop plan.
    pub(crate) empty: bool,
    /// The last shared collection mark epoch that reached this block.
    pub(crate) mark_epoch: u64,
}

/// One stable shared heap large-block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct LargeBlockId(u64);

impl LargeBlockId {
    /// Create one shared heap large-block identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the shared heap large-block identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based large-block slot index.
    pub(crate) fn index(self) -> HeapResult<usize> {
        let Some(index) = self.0.checked_sub(1) else {
            return Err(HeapError::representation(
                HeapRepresentationError::InvalidLargeBlockId { id: self.0 },
            ));
        };

        Ok(index as usize)
    }
}

/// One frozen shared heap large-block image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct LargeBlockImage {
    /// The first byte offset inside shared heap storage.
    pub first_offset: usize,
    /// The logical byte length of this block.
    pub byte_len: usize,
    /// The trace map for this block.
    pub trace_map: TraceMap,
    /// The drop plan for this managed block.
    pub drop: Option<DropPlan>,
    /// Whether a borrow or heap storage retained this block.
    pub retained: bool,
    /// Whether this block's values moved out.
    pub empty: bool,
}
