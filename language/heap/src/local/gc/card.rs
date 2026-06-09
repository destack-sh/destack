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

        // mark the covered card span in one bitmap update
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

    /// Find one dirty card at or after the given card index.
    pub(crate) fn find_dirty_card(&self, card_index: usize) -> Option<DirtyCard> {
        let index = self.dirty.first_set_from(card_index)?;
        let byte_start = index * DEFAULT_CARD_SIZE_BYTES;
        let byte_end = (byte_start + DEFAULT_CARD_SIZE_BYTES).min(self.byte_len);

        Some(DirtyCard {
            index,
            byte_start,
            byte_len: byte_end - byte_start,
        })
    }

    /// Clear one dirty card.
    pub(crate) fn clear_card(&mut self, card_index: usize) {
        self.dirty.clear(card_index);
    }
}

/// One dirty remembered-set card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirtyCard {
    /// The dirty card index.
    pub(crate) index: usize,
    /// The card byte start within its owning extent.
    pub(crate) byte_start: usize,
    /// The card byte length.
    pub(crate) byte_len: usize,
}

impl DirtyCard {
    /// Return each span slot overlapped by this dirty card.
    pub(crate) fn slot_overlaps(
        self,
        size_class: usize,
        slot_count: usize,
    ) -> impl Iterator<Item = CardSlotOverlap> {
        // translate the dirty byte card to covered span slots
        let card_end = self.byte_start + self.byte_len;
        let first_slot = self.byte_start / size_class;
        let last_slot = (card_end - 1) / size_class;
        let end_slot = (last_slot + 1).min(slot_count);

        (first_slot..end_slot).filter_map(move |slot_index| {
            // intersect the card range with this slot payload
            let slot_start = size_class * slot_index;
            let slot_end = slot_start + size_class;
            let overlap_start = self.byte_start.max(slot_start);
            let overlap_end = card_end.min(slot_end);
            if overlap_start >= overlap_end {
                return None;
            }

            Some(CardSlotOverlap {
                slot_index,
                slot_start,
                byte_start: overlap_start - slot_start,
                byte_len: overlap_end - overlap_start,
            })
        })
    }
}

/// One dirty-card overlap with one span slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CardSlotOverlap {
    /// The overlapped slot index.
    pub(crate) slot_index: usize,
    /// The slot base byte offset inside the span.
    pub(crate) slot_start: usize,
    /// The overlap start inside the slot payload.
    pub(crate) byte_start: usize,
    /// The overlap byte length.
    pub(crate) byte_len: usize,
}
