use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Number of bits stored per backing word.
const WORD_BITS: usize = u64::BITS as usize;

/// A fixed-length set of bits packed into 64-bit words.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct BitSet {
    /// Backing words, each holding 64 bits from low to high.
    words: Vec<u64>,
    /// The number of valid bits; positions at or beyond this stay clear.
    length: usize,
}

impl BitSet {
    /// Create a bitset of `length` bits, all clear.
    pub fn new(length: usize) -> Self {
        Self {
            words: vec![0; length.div_ceil(WORD_BITS)],
            length,
        }
    }

    /// Return the number of bits the set holds.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Return whether no bits are set.
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|word| *word == 0)
    }

    /// Return whether the bit at `index` is set.
    pub fn contains(&self, index: usize) -> bool {
        debug_assert!(index < self.length, "bit index out of range");
        let (word, bit) = locate(index);
        self.words[word] & (1 << bit) != 0
    }

    /// Set the bit at `index`, returning whether it was previously clear.
    pub fn insert(&mut self, index: usize) -> bool {
        debug_assert!(index < self.length, "bit index out of range");
        let (word, bit) = locate(index);
        let mask = 1u64 << bit;
        let was_clear = self.words[word] & mask == 0;
        self.words[word] |= mask;

        was_clear
    }

    /// Clear the bit at `index`, returning whether it was previously set.
    pub fn remove(&mut self, index: usize) -> bool {
        debug_assert!(index < self.length, "bit index out of range");
        let (word, bit) = locate(index);
        let mask = 1u64 << bit;
        let was_set = self.words[word] & mask != 0;
        self.words[word] &= !mask;

        was_set
    }

    /// Count the set bits.
    pub fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Insert every bit from another equally sized set.
    pub fn union_with(&mut self, other: &Self) -> bool {
        debug_assert_eq!(self.length, other.length);
        let mut changed = false;

        for (word, other) in self.words.iter_mut().zip(&other.words) {
            let merged = *word | *other;
            changed |= merged != *word;
            *word = merged;
        }

        changed
    }

    /// Retain every bit contained in another equally sized set.
    pub fn intersect_with(&mut self, other: &Self) {
        debug_assert_eq!(self.length, other.length);

        for (word, other) in self.words.iter_mut().zip(&other.words) {
            *word &= *other;
        }
    }

    /// Remove every bit contained in another equally sized set.
    pub fn subtract(&mut self, other: &Self) {
        debug_assert_eq!(self.length, other.length);

        for (word, other) in self.words.iter_mut().zip(&other.words) {
            *word &= !*other;
        }
    }

    /// Iterate set bit indices in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .flat_map(|(word_index, word)| {
                let mut word = *word;

                std::iter::from_fn(move || {
                    if word == 0 {
                        return None;
                    }

                    let bit = word.trailing_zeros() as usize;
                    word &= word - 1;

                    Some(word_index * WORD_BITS + bit)
                })
            })
    }
}

/// Split a bit index into its backing word and in-word offset.
fn locate(index: usize) -> (usize, usize) {
    (index / WORD_BITS, index % WORD_BITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_query_across_word_boundary() {
        // exercise bits in the first and second backing words
        let mut bits = BitSet::new(130);
        assert!(bits.insert(0));
        assert!(bits.insert(63));
        assert!(bits.insert(64));
        assert!(bits.insert(129));

        // a second insert reports the bit was already set
        assert!(!bits.insert(64));

        // set bits read back true, clear bits false
        assert!(bits.contains(0) && bits.contains(63) && bits.contains(64) && bits.contains(129));
        assert!(!bits.contains(1) && !bits.contains(65) && !bits.contains(128));
        assert_eq!(bits.count(), 4);
    }

    #[test]
    fn test_roundtrip_through_serde() {
        // a populated bitset survives a serialize/deserialize cycle exactly
        let mut bits = BitSet::new(200);
        for index in [3, 64, 65, 199] {
            bits.insert(index);
        }
        let encoded = serde_json::to_string(&bits).unwrap();
        let decoded: BitSet = serde_json::from_str(&encoded).unwrap();

        assert_eq!(bits, decoded);
    }
}
