use crate::PageView;

/// One logical shared raw-space entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRawEntry {
    /// Whether this entry id is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this entry.
    pub(crate) len: usize,
    /// The allocator pages for this entry.
    pub(crate) pages: PageView,
}

impl SharedRawEntry {
    /// Return one vacant shared entry slot.
    pub(crate) const fn vacant() -> Self {
        Self {
            is_live: false,
            len: 0,
            pages: PageView::empty(),
        }
    }

    /// Create one live shared entry.
    pub(crate) const fn new(len: usize, pages: PageView) -> Self {
        Self {
            is_live: true,
            len,
            pages,
        }
    }

    /// Report whether this shared entry slot is vacant.
    pub(crate) const fn is_vacant(&self) -> bool {
        !self.is_live
    }

    /// Retire this shared entry slot.
    pub(crate) fn retire(&mut self) {
        *self = Self::vacant();
    }
}
