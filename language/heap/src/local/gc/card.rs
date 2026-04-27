use crate::allocator::Bitmap;
use crate::local::constants::DEFAULT_CARD_BYTES;

/// One card set for mature remembered regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardSet {
    /// The logical byte length covered by this card set.
    byte_len: usize,
    /// The byte width covered by each remembered card.
    card_bytes: usize,
    /// The dirty cards keyed by card index.
    dirty: Bitmap,
}

impl CardSet {
    /// Create one empty card set for the given byte length.
    pub(crate) fn with_len(byte_len: usize) -> Self {
        let card_count = byte_len.div_ceil(DEFAULT_CARD_BYTES);

        Self {
            byte_len,
            card_bytes: DEFAULT_CARD_BYTES,
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
        let start_card = start / self.card_bytes;
        let end_card = end.div_ceil(self.card_bytes);

        // mark the covered card run in one bitmap update
        self.dirty.set_range(start_card, end_card - start_card);
    }

    /// Clear every dirty card.
    pub(crate) fn clear(&mut self) {
        self.dirty.clear_all();
    }

    /// Visit each dirty card range.
    pub(crate) fn visit_dirty_ranges(&self, mut callback: impl FnMut(usize, usize)) {
        // walk each contiguous dirty card run in order
        self.dirty.visit_set_ranges(|start_card, card_count| {
            let start = start_card * self.card_bytes;
            let end = ((start_card + card_count) * self.card_bytes).min(self.byte_len);

            callback(start, end - start);
        });
    }
}
