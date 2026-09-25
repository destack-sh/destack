use serde::{Deserialize, Serialize};
use tspp_memory::MemoryRange;
use tspp_mir::TraceMap;
use tspp_serde::Reflect;

use crate::{DropPlan, HeapError, HeapRepresentationError, HeapResult};

/// One live heap large block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeBlock {
    /// The first byte offset inside heap storage.
    pub(crate) first_offset: usize,
    /// The logical byte length of this block.
    pub(crate) byte_len: usize,
    /// The memory pages for this block.
    pub(crate) pages: MemoryRange,
    /// The trace map for this block.
    pub(crate) trace_map: TraceMap,
    /// The drop plan for this managed block.
    pub(crate) drop: Option<DropPlan>,
    /// Whether managed storage referenced this block, its release waiting for the collector.
    pub(crate) retained: bool,
    /// Whether the released block's values moved out, freeing it without its drop plan.
    pub(crate) empty: bool,
    /// The mark epoch when this block was last marked.
    pub(crate) mark_epoch: u64,
}

/// One heap large-block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct LargeBlockId(u64);

impl LargeBlockId {
    /// Create one heap large-block identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the heap large-block identifier value.
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

/// One frozen heap large-block image.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) struct LargeBlockImage {
    /// The first byte offset inside heap storage.
    pub first_offset: usize,
    /// The logical byte length of this block.
    pub byte_len: usize,
    /// The trace map for this block.
    pub trace_map: TraceMap,
    /// The drop plan for this managed block.
    pub drop: Option<DropPlan>,
    /// Whether managed storage referenced this block.
    pub retained: bool,
    /// Whether the released block's values moved out.
    pub empty: bool,
}
