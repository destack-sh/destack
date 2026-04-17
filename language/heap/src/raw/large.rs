use serde::{Deserialize, Serialize};

use crate::alloc::PageView;

/// One frozen raw large-entry root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LargeEntryImage {
    /// Whether this entry slot is live.
    pub is_live: bool,
    /// The logical byte length of this entry.
    pub len: usize,
    /// The arena pages for this entry.
    pub pages: PageView,
}

/// One stable raw large-entry identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LargeEntryId(u64);

impl LargeEntryId {
    /// Create one raw large-entry identifier.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Return the raw large-entry identifier value.
    pub const fn id(self) -> u64 {
        self.0
    }

    /// Return the zero-based large-entry slot index.
    pub fn index(self) -> crate::HeapResult<usize> {
        let Some(index) = self.0.checked_sub(1) else {
            return Err(crate::HeapError::InvalidLargeEntryId { id: self.0 });
        };

        Ok(index as usize)
    }
}

/// One live raw large entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LargeEntry {
    /// Whether this large-entry slot is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this entry.
    pub(crate) len: usize,
    /// The arena pages for this entry.
    pub(crate) pages: PageView,
}

impl LargeEntry {
    /// Retire this raw large-entry slot.
    pub(crate) fn retire(&mut self) {
        self.is_live = false;
        self.len = 0;
        self.pages = PageView::empty();
    }
}
