use crate::PageSpan;

/// One logical shared raw-space block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SharedRawBlock {
    /// Whether this block slot is live.
    pub(crate) is_live: bool,
    /// The first byte offset inside shared raw space.
    pub(crate) first_offset: usize,
    /// The logical byte length of this block.
    pub(crate) byte_len: usize,
    /// The allocator pages for this block.
    pub(crate) pages: PageSpan,
}

impl SharedRawBlock {
    /// Return one vacant shared block slot.
    pub(crate) const fn vacant() -> Self {
        Self {
            is_live: false,
            first_offset: 0,
            byte_len: 0,
            pages: PageSpan::empty(),
        }
    }

    /// Create one live shared block.
    pub(crate) const fn new(first_offset: usize, byte_len: usize, pages: PageSpan) -> Self {
        Self {
            is_live: true,
            first_offset,
            byte_len,
            pages,
        }
    }

    /// Report whether this shared block slot is vacant.
    pub(crate) const fn is_vacant(&self) -> bool {
        !self.is_live
    }

    /// Retire this shared block slot.
    pub(crate) fn retire(&mut self) {
        *self = Self::vacant();
    }
}
