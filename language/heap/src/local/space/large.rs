use serde::{Deserialize, Serialize};

use destack_mir::LayoutId;

use super::CardSet;
use crate::allocator::PageView;
use crate::{HeapError, HeapResult};

/// One frozen heap large-entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeEntryImage {
    /// Whether this entry slot is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The allocator pages for this entry.
    pub pages: PageView,
    /// The managed layout stored in this entry.
    pub layout_id: LayoutId,
}

/// One heap large-entry identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeEntryId(u64);

impl LargeEntryId {
    /// Create one heap large-entry identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the heap large-entry identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based large-entry slot index.
    pub(crate) fn index(self) -> HeapResult<usize> {
        let Some(index) = self.0.checked_sub(1) else {
            return Err(HeapError::InvalidLargeEntryId { id: self.0 });
        };

        Ok(index as usize)
    }
}

/// One live heap large entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeEntry {
    /// Whether this large-entry slot is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this entry.
    pub(crate) len: usize,
    /// The allocator pages for this entry.
    pub(crate) pages: PageView,
    /// The managed layout stored in this entry.
    pub(crate) layout_id: LayoutId,
    /// Whether this entry is marked in the active cycle.
    pub(crate) is_marked: bool,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this entry is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl LargeEntry {
    /// Retire this heap large-entry slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.len = 0;
        self.pages = PageView::empty();
        self.is_marked = false;
        self.dirty_cards.clear();
        self.is_dirty_queued = false;
    }
}
