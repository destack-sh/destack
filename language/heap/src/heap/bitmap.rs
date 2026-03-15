use serde::{Deserialize, Serialize};

/// The number of bits in one bitmap word.
const BITMAP_WORD_BITS: usize = u64::BITS as usize;

/// One bitmap for run-sized metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bitmap {
    /// The logical bit capacity.
    capacity: usize,
    /// The packed bitmap words.
    words: Vec<u64>,
}

impl Bitmap {
    /// Create one empty bitmap with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let word_count = capacity.div_ceil(BITMAP_WORD_BITS);

        Self {
            capacity,
            words: vec![0; word_count],
        }
    }

    /// Report whether one bit is set.
    pub fn contains(&self, offset: usize) -> bool {
        if offset >= self.capacity {
            return false;
        }

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index] & mask != 0
    }

    /// Set one bit.
    pub fn set(&mut self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index] |= mask;
    }

    /// Clear one bit.
    pub fn clear(&mut self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.words[word_index] &= mask;
    }

    /// Clear all bits.
    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    /// Count the number of set bits.
    pub fn count_ones(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Return the first clear bit from the given offset.
    pub fn first_clear_from(&self, start: usize) -> Option<usize> {
        if start >= self.capacity {
            return None;
        }

        (start..self.capacity).find(|&index| !self.contains(index))
    }

    /// Return the retained bytes for this bitmap.
    pub fn retained_bytes(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>()
    }
}
