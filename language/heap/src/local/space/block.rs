use serde::{Deserialize, Serialize};

use destack_mir::TraceMap;

use super::CardSet;
use crate::allocator::PageSpan;
use crate::{HeapError, HeapRepresentationError, HeapResult};

/// One live heap large block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeBlock {
    /// Whether this large-block slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside heap space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this block.
    pub(crate) byte_len: usize,
    /// The allocator pages for this block.
    pub(crate) pages: PageSpan,
    /// The trace map for this block.
    pub(crate) trace_map: TraceMap,
    /// The mark epoch when this block was last marked.
    pub(crate) mark_epoch: u64,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this block is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl LargeBlock {
    /// Retire this heap large-block slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.first_offset = 0;
        self.byte_len = 0;
        self.pages = PageSpan::empty();
        self.mark_epoch = 0;
        self.dirty_cards.clear();
        self.is_dirty_queued = false;
    }
}

/// One heap large-block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeBlockImage {
    /// Whether this block slot is live.
    pub is_live: bool,
    /// The first byte offset inside heap space.
    pub first_offset: usize,
    /// The logical byte length of this block.
    pub byte_len: usize,
    /// The captured block bytes.
    pub bytes: Box<[u8]>,
    /// The trace map for this block.
    pub trace_map: TraceMap,
}
