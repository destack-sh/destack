use serde::{Deserialize, Serialize};

use destack_mir::TraceMap;

use crate::allocator::PageSpan;
use crate::{HeapError, HeapRepresentationError, HeapResult};

/// One live shared heap large block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeBlock {
    /// Whether this large-block slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside shared heap storage.
    pub(crate) first_offset: usize,
    /// The logical byte length of this block.
    pub(crate) byte_len: usize,
    /// The allocator pages for this block.
    pub(crate) pages: PageSpan,
    /// The trace map for this block.
    pub(crate) trace_map: TraceMap,
    /// The last shared collection mark epoch that reached this block.
    pub(crate) mark_epoch: u64,
}

impl LargeBlock {
    /// Retire this shared heap large-block slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.first_offset = 0;
        self.byte_len = 0;
        self.pages = PageSpan::empty();
        self.mark_epoch = 0;
    }
}

/// One stable shared heap large-block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeBlockImage {
    /// Whether this block slot is live.
    pub is_live: bool,
    /// The first byte offset inside shared heap storage.
    pub first_offset: usize,
    /// The logical byte length of this block.
    pub byte_len: usize,
    /// The captured block bytes.
    pub bytes: Box<[u8]>,
    /// The trace map for this block.
    pub trace_map: TraceMap,
}
