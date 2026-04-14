use super::Bitmap;
use crate::DEFAULT_CARD_BYTES;

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
        let card_count = byte_len.div_ceil(DEFAULT_CARD_BYTES);

        Self {
            byte_len,
            dirty: Bitmap::with_capacity(card_count),
        }
    }
}
