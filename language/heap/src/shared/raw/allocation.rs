use crate::PageView;

/// One logical shared raw-space allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRawAllocation {
    /// Whether this allocation slot is live.
    pub(crate) is_live: bool,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The allocator pages for this allocation.
    pub(crate) pages: PageView,
}

impl SharedRawAllocation {
    /// Return one vacant shared allocation slot.
    pub(crate) const fn vacant() -> Self {
        Self {
            is_live: false,
            len: 0,
            pages: PageView::empty(),
        }
    }

    /// Create one live shared allocation.
    pub(crate) const fn new(len: usize, pages: PageView) -> Self {
        Self {
            is_live: true,
            len,
            pages,
        }
    }

    /// Report whether this shared allocation slot is vacant.
    pub(crate) const fn is_vacant(&self) -> bool {
        !self.is_live
    }

    /// Retire this shared allocation slot.
    pub(crate) fn retire(&mut self) {
        *self = Self::vacant();
    }
}
