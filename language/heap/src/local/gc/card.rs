use crate::DEFAULT_CARD_SIZE_BYTES;
use crate::allocator::Bitmap;

/// One card set for mature remembered regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardSet {
    /// The logical byte length covered by this card set.
    byte_len: usize,
    /// The dirty cards keyed by card index.
    dirty: Bitmap,
}

impl CardSet {
    /// Create one empty card set for the given byte length.
    pub(crate) fn with_len(byte_len: usize) -> Self {
        let card_count = byte_len.div_ceil(DEFAULT_CARD_SIZE_BYTES);

        Self {
            byte_len,
            dirty: Bitmap::with_capacity(card_count),
        }
    }

    /// Mark one byte range dirty.
    pub(crate) fn mark_range(&mut self, start: usize, len: usize) {
        // ignore empty or out-of-bounds ranges
        if len == 0 || start >= self.byte_len {
            return;
        }

        let end = (start + len).min(self.byte_len);
        let start_card = start / DEFAULT_CARD_SIZE_BYTES;
        let end_card = end.div_ceil(DEFAULT_CARD_SIZE_BYTES);

        // mark the covered card run in one bitmap update
        self.dirty.set_range(start_card, end_card - start_card);
    }

    /// Clear every dirty card.
    pub(crate) fn clear(&mut self) {
        self.dirty.clear_all();
    }

    /// Return whether this card set has no dirty cards.
    pub(crate) fn is_empty(&self) -> bool {
        self.dirty.first_set_from(0).is_none()
    }

    /// Return one dirty card at or after the given card index.
    pub(crate) fn next_dirty_card_from(&self, card_index: usize) -> Option<(usize, usize, usize)> {
        let card_index = self.dirty.first_set_from(card_index)?;
        let start = card_index * DEFAULT_CARD_SIZE_BYTES;
        let end = (start + DEFAULT_CARD_SIZE_BYTES).min(self.byte_len);

        Some((card_index, start, end - start))
    }

    /// Clear one dirty card.
    pub(crate) fn clear_card(&mut self, card_index: usize) {
        self.dirty.clear(card_index);
    }
}
