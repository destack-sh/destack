use crate::DEFAULT_CARD_BYTES;
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
        let card_count = byte_len.div_ceil(DEFAULT_CARD_BYTES);

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
        let start_card = start / DEFAULT_CARD_BYTES;
        let end_card = end.div_ceil(DEFAULT_CARD_BYTES);

        // mark the covered card run in one bitmap update
        self.dirty.set_range(start_card, end_card - start_card);
    }

    /// Clear every dirty card.
    pub(crate) fn clear(&mut self) {
        self.dirty.clear_all();
    }

    /// Return each dirty byte range.
    pub(crate) fn dirty_ranges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.dirty.set_ranges().map(|(start_card, card_count)| {
            let start = start_card * DEFAULT_CARD_BYTES;
            let end = ((start_card + card_count) * DEFAULT_CARD_BYTES).min(self.byte_len);

            (start, end - start)
        })
    }
}
