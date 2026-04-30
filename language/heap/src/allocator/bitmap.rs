use serde::{Deserialize, Serialize};

/// The number of bits in one bitmap word.
const BITMAP_WORD_BITS: usize = u64::BITS as usize;

/// One bitmap for span-sized metadata.
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

    /// Ensure this bitmap can represent the given bit count.
    pub fn ensure_capacity(&mut self, capacity: usize) {
        if capacity <= self.capacity {
            return;
        }

        let word_count = capacity.div_ceil(BITMAP_WORD_BITS);

        self.words.resize(word_count, 0);
        self.capacity = capacity;
    }

    /// Return the logical bit capacity.
    pub const fn capacity(&self) -> usize {
        self.capacity
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
    #[inline(always)]
    pub fn set(&mut self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index] |= mask;
    }

    /// Set one bit that the caller already knows is inside this bitmap.
    #[inline(always)]
    pub(crate) fn set_in_bounds(&mut self, offset: usize) {
        debug_assert!(offset < self.capacity);

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.words[word_index] |= mask;
    }

    /// Set every bit inside the given range.
    pub fn set_range(&mut self, start: usize, len: usize) {
        if len == 0 || start >= self.capacity {
            return;
        }

        let end = (start + len).min(self.capacity);
        let start_word_index = start / BITMAP_WORD_BITS;
        let end_word_index = (end - 1) / BITMAP_WORD_BITS;
        let start_bit_offset = start % BITMAP_WORD_BITS;
        let end_bit_offset = end % BITMAP_WORD_BITS;

        // handle the single-word case directly
        if start_word_index == end_word_index {
            let range_mask = word_range_mask(start_bit_offset, end_bit_offset);
            self.words[start_word_index] |= range_mask;

            return;
        }

        // fill the partial first word
        self.words[start_word_index] |= !low_bit_mask(start_bit_offset);

        // fill any fully covered middle words
        for word_index in start_word_index + 1..end_word_index {
            self.words[word_index] = u64::MAX;
        }

        // fill the partial last word
        let last_word_mask = if end_bit_offset == 0 {
            u64::MAX
        } else {
            low_bit_mask(end_bit_offset)
        };
        self.words[end_word_index] |= last_word_mask;
    }

    /// Clear one bit.
    #[inline(always)]
    pub fn clear(&mut self, offset: usize) {
        if offset >= self.capacity {
            return;
        }

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.words[word_index] &= mask;
    }

    /// Clear one bit that the caller already knows is inside this bitmap.
    #[inline(always)]
    pub(crate) fn clear_in_bounds(&mut self, offset: usize) {
        debug_assert!(offset < self.capacity);

        let word_index = offset / BITMAP_WORD_BITS;
        let bit_offset = offset % BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.words[word_index] &= mask;
    }

    /// Clear all bits.
    pub fn clear_all(&mut self) {
        self.words.fill(0);
    }

    /// Clear every bit inside the given range.
    pub fn clear_range(&mut self, start: usize, len: usize) {
        if len == 0 || start >= self.capacity {
            return;
        }

        let end = (start + len).min(self.capacity);
        let start_word_index = start / BITMAP_WORD_BITS;
        let end_word_index = (end - 1) / BITMAP_WORD_BITS;
        let start_bit_offset = start % BITMAP_WORD_BITS;
        let end_bit_offset = end % BITMAP_WORD_BITS;

        // handle the single-word case directly
        if start_word_index == end_word_index {
            let range_mask = !word_range_mask(start_bit_offset, end_bit_offset);
            self.words[start_word_index] &= range_mask;

            return;
        }

        // clear the partial first word
        self.words[start_word_index] &= low_bit_mask(start_bit_offset);

        // clear any fully covered middle words
        for word_index in start_word_index + 1..end_word_index {
            self.words[word_index] = 0;
        }

        // clear the partial last word
        let last_word_mask = if end_bit_offset == 0 {
            0
        } else {
            !low_bit_mask(end_bit_offset)
        };
        self.words[end_word_index] &= last_word_mask;
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

        let mut word_index = start / BITMAP_WORD_BITS;
        let bit_offset = start % BITMAP_WORD_BITS;
        let mut word = self.words[word_index] | low_bit_mask(bit_offset);

        loop {
            let available_bits = !word;
            if available_bits != 0 {
                let first_bit = available_bits.trailing_zeros() as usize;
                let index = word_index * BITMAP_WORD_BITS + first_bit;

                if index < self.capacity {
                    return Some(index);
                }

                return None;
            }

            word_index += 1;
            if word_index >= self.words.len() {
                return None;
            }

            word = self.words[word_index];
        }
    }

    /// Return the first set bit from the given offset.
    pub fn first_set_from(&self, start: usize) -> Option<usize> {
        if start >= self.capacity {
            return None;
        }

        let mut word_index = start / BITMAP_WORD_BITS;
        let bit_offset = start % BITMAP_WORD_BITS;
        let mut word = self.words[word_index] & !low_bit_mask(bit_offset);

        loop {
            if word != 0 {
                let first_bit = word.trailing_zeros() as usize;
                let index = word_index * BITMAP_WORD_BITS + first_bit;

                if index < self.capacity {
                    return Some(index);
                }

                return None;
            }

            word_index += 1;
            if word_index >= self.words.len() {
                return None;
            }

            word = self.words[word_index];
        }
    }

    /// Return the last set bit at or before the given offset.
    pub fn last_set_at_or_before(&self, start: usize) -> Option<usize> {
        if self.capacity == 0 {
            return None;
        }

        let start = start.min(self.capacity - 1);
        let mut word_index = start / BITMAP_WORD_BITS;
        let bit_offset = start % BITMAP_WORD_BITS;
        let mut word = self.words[word_index] & low_bit_mask(bit_offset + 1);

        loop {
            if word != 0 {
                let last_bit = BITMAP_WORD_BITS - 1 - word.leading_zeros() as usize;
                let index = word_index * BITMAP_WORD_BITS + last_bit;

                return Some(index);
            }

            if word_index == 0 {
                return None;
            }

            word_index -= 1;
            word = self.words[word_index];
        }
    }

    /// Return the retained bytes for this bitmap.
    pub fn retained_bytes(&self) -> usize {
        self.words.capacity() * std::mem::size_of::<u64>()
    }

    /// Visit each contiguous set-bit range.
    pub fn visit_set_ranges(&self, mut callback: impl FnMut(usize, usize)) {
        let mut start = 0usize;

        // walk each retained set-bit run in order
        while let Some(range_start) = self.first_set_from(start) {
            let range_end = self.first_clear_from(range_start).unwrap_or(self.capacity);

            callback(range_start, range_end - range_start);
            start = range_end;
        }
    }
}

/// Return one mask with every low bit below the offset set.
fn low_bit_mask(bit_offset: usize) -> u64 {
    if bit_offset >= BITMAP_WORD_BITS {
        u64::MAX
    } else if bit_offset == 0 {
        0
    } else {
        (1_u64 << bit_offset) - 1
    }
}

/// Return one mask that covers the half-open bit range inside one word.
fn word_range_mask(start_bit_offset: usize, end_bit_offset: usize) -> u64 {
    let low_mask = low_bit_mask(start_bit_offset);
    let high_mask = if end_bit_offset == 0 {
        u64::MAX
    } else {
        low_bit_mask(end_bit_offset)
    };

    high_mask & !low_mask
}
