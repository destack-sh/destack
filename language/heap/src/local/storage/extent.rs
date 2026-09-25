use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::LargeBlockId;
use crate::{HeapError, HeapReference, HeapResult, Slot};

/// One heap allocation place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum HeapPlace {
    /// One small block stored in one span slot.
    Slot(Slot),
    /// One block stored in heap large space.
    LargeBlock(LargeBlockId),
}

/// The allocation metadata owning one local heap page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub(crate) enum PageOwner {
    /// One span page and its logical page index.
    Span {
        /// The owning span index.
        span_index: usize,
        /// The logical page index inside the span.
        logical_page_index: usize,
    },
    /// One large-block page and its logical page index.
    LargeBlock {
        /// The owning large-block id.
        block_id: LargeBlockId,
        /// The logical page index inside the large block.
        logical_page_index: usize,
    },
}

/// One resolved heap extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeapExtent {
    /// The heap allocation place.
    pub(crate) place: HeapPlace,
    /// The base reference for the owning block.
    pub(crate) base: HeapReference,
    /// The byte offset from the base block.
    pub(crate) byte_offset: usize,
    /// The logical byte length for the owning block.
    pub(crate) byte_len: usize,
}

impl HeapExtent {
    /// Project one visible byte range into this allocation.
    pub(crate) fn project(self, start: usize, len: usize) -> HeapResult<usize> {
        debug_assert!(self.byte_offset <= self.byte_len);

        // validate the caller start relative to the visible payload
        let remaining = self.byte_len - self.byte_offset;
        if start > remaining {
            return Err(HeapError::InvalidByteRange {
                start,
                len,
                capacity: self.byte_len,
            });
        }

        // validate the caller length after projecting the start
        let byte_offset = self.byte_offset + start;
        let remaining = self.byte_len - byte_offset;
        if len > remaining {
            return Err(HeapError::InvalidByteRange {
                start: byte_offset,
                len,
                capacity: self.byte_len,
            });
        }

        Ok(byte_offset)
    }
}
