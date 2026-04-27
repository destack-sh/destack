use crate::PageRun;

/// One logical shared raw-space allocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRawAllocation {
    /// Whether this allocation slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside shared raw space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this allocation.
    pub(crate) len: usize,
    /// The allocator pages for this allocation.
    pub(crate) pages: PageRun,
}

impl SharedRawAllocation {
    /// Return one vacant shared allocation slot.
    pub(crate) const fn vacant() -> Self {
        Self {
            is_live: false,
            first_offset: 0,
            len: 0,
            pages: PageRun::empty(),
        }
    }

    /// Create one live shared allocation.
    pub(crate) const fn new(first_offset: usize, len: usize, pages: PageRun) -> Self {
        Self {
            is_live: true,
            first_offset,
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
