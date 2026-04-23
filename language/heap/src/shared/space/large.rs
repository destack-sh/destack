use serde::{Deserialize, Serialize};

use crate::allocator::PageView;
use crate::{HeapError, HeapResult, ShapeId};

/// One frozen shared heap large-entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapLargeEntryImage {
    /// Whether this entry slot is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The allocator pages for this entry.
    pub pages: PageView,
    /// The interned entry shape for this entry.
    pub shape_id: u32,
}

/// One stable shared heap large-entry identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SharedLargeEntryId(u64);

impl SharedLargeEntryId {
    /// Create one shared heap large-entry identifier.
    pub(crate) const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the shared heap large-entry identifier value.
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

/// One live shared heap large entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedLargeEntry {
    /// Whether this large-entry slot is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this entry.
    pub(crate) len: usize,
    /// The allocator pages for this entry.
    pub(crate) pages: PageView,
    /// The interned entry shape for this entry.
    pub(crate) shape_id: ShapeId,
    /// Whether this entry is marked in the active cycle.
    pub(crate) is_marked: bool,
}

impl SharedLargeEntry {
    /// Retire this shared heap large-entry slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.len = 0;
        self.pages = PageView::empty();
        self.is_marked = false;
    }
}
