use destack_mir::LayoutId;
use serde::{Deserialize, Serialize};

use super::{ReferenceMapId, StoredLayoutId};
use crate::alloc::{CardSet, PageMap};

/// One frozen managed allocation root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AllocationImage {
    /// Whether this allocation slot is currently allocated.
    pub is_allocated: bool,
    /// The logical byte length of this allocation.
    pub len: usize,
    /// The arena pages for this allocation.
    pub pages: PageMap,
    /// The interned reference map for this allocation.
    pub trace_id: ReferenceMapId,
    /// The durable layout id for this allocation, if any.
    pub layout_id: Option<LayoutId>,
}

/// One stable managed allocation identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AllocationId(u64);

impl AllocationId {
    /// Create one managed allocation identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the managed allocation identifier value.
    pub(crate) const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based allocation slot index.
    pub(crate) const fn index(self) -> usize {
        self.0.saturating_sub(1) as usize
    }
}

/// One live managed allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Allocation {
    /// Whether this allocation slot is currently allocated.
    pub(crate) is_allocated: bool,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The arena pages for this allocation.
    pub(crate) pages: PageMap,
    /// The interned reference map for this allocation.
    pub(crate) trace_id: ReferenceMapId,
    /// The durable layout id for this allocation, if any.
    pub(crate) layout_id: StoredLayoutId,
    /// The live mark state for this allocation.
    pub(crate) marked: bool,
    /// The active pin count for this allocation.
    pub(crate) pin_count: u16,
    /// The dirty cards remembered for young tracing.
    pub(crate) dirty_cards: CardSet,
    /// Whether this allocation is already queued for dirty-card scanning.
    pub(crate) is_dirty_queued: bool,
}
