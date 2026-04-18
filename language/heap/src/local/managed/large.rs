use serde::{Deserialize, Serialize};

use super::{CardSet, EdgeId};
use crate::LayoutId;
use crate::arena::PageView;

/// One frozen managed large-entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeEntryImage {
    /// Whether this entry slot is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The arena pages for this entry.
    pub pages: PageView,
    /// The interned edge map for this entry.
    pub edge_id: EdgeId,
    /// The durable layout id for this entry, if any.
    pub layout_id: Option<LayoutId>,
}

/// One stable managed large-entry identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeEntryId(u64);

impl LargeEntryId {
    /// Create one managed large-entry identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the managed large-entry identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based large-entry slot index.
    pub(crate) fn index(self) -> crate::HeapResult<usize> {
        let Some(index) = self.0.checked_sub(1) else {
            return Err(crate::HeapError::InvalidLargeEntryId { id: self.0 });
        };

        Ok(index as usize)
    }
}

/// One live managed large entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeEntry {
    /// Whether this large-entry slot is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this entry.
    pub(crate) len: usize,
    /// The arena pages for this entry.
    pub(crate) pages: PageView,
    /// The interned edge map for this entry.
    pub(crate) edge_id: EdgeId,
    /// The durable layout id for this entry, if any.
    pub(crate) layout_id: Option<LayoutId>,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this entry is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}

impl LargeEntry {
    /// Retire this managed large-entry slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.len = 0;
        self.pages = PageView::empty();
        self.edge_id = EdgeId::empty();
        self.layout_id = None;
        self.dirty_cards.clear();
        self.is_dirty_queued = false;
    }
}
