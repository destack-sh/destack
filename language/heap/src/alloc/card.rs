use super::Bitmap;

/// The fixed byte width for one remembered-set card.
pub(crate) const CARD_BYTES: usize = 256;

/// One dirty-card set for mature remembered regions.
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
        let card_count = byte_len.div_ceil(CARD_BYTES);

        Self {
            byte_len,
            dirty: Bitmap::with_capacity(card_count),
        }
    }

    /// Report whether this card set has any dirty cards.
    pub(crate) fn has_dirty_cards(&self) -> bool {
        self.dirty.count_ones() > 0
    }

    /// Clear all dirty cards.
    pub(crate) fn clear_all(&mut self) {
        self.dirty.clear_all();
    }

    /// Mark one byte range dirty.
    pub(crate) fn mark_range(&mut self, start: usize, len: usize) -> bool {
        if len == 0 || start >= self.byte_len {
            return false;
        }

        let Some(end) = start.checked_add(len) else {
            return false;
        };
        let end = end.min(self.byte_len);
        let first_card = start / CARD_BYTES;
        let last_card = (end - 1) / CARD_BYTES;
        let mut marked_any = false;

        for card_index in first_card..=last_card {
            if self.dirty.contains(card_index) {
                continue;
            }

            self.dirty.set(card_index);
            marked_any = true;
        }

        marked_any
    }

    /// Mark one card dirty.
    pub(crate) fn mark_card(&mut self, card_index: usize) {
        self.dirty.set(card_index);
    }

    /// Clear one card.
    pub(crate) fn clear_card(&mut self, card_index: usize) {
        self.dirty.clear(card_index);
    }

    /// Return the first dirty card from the given index.
    pub(crate) fn first_dirty_from(&self, start: usize) -> Option<usize> {
        self.dirty.first_set_from(start)
    }

    /// Return the byte start for the given card.
    pub(crate) fn card_start(&self, card_index: usize) -> usize {
        card_index * CARD_BYTES
    }

    /// Return the logical byte length for the given card.
    pub(crate) fn card_len(&self, card_index: usize) -> usize {
        let start = self.card_start(card_index);

        self.byte_len.saturating_sub(start).min(CARD_BYTES)
    }

    /// Return the retained bytes for this card set.
    pub(crate) fn retained_bytes(&self) -> usize {
        self.dirty.retained_bytes()
    }
}
