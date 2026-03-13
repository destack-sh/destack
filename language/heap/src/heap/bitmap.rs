use serde::de::{Error as _, SeqAccess, Visitor};
use serde::ser::SerializeTuple;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The number of bits in one bitmap word.
const PAGE_BITMAP_WORD_BITS: usize = u64::BITS as usize;

/// Compute the number of words needed for a bitmap of the given capacity.
const fn bitmap_word_count(capacity: usize) -> usize {
    assert!(capacity.is_multiple_of(PAGE_BITMAP_WORD_BITS));
    capacity / PAGE_BITMAP_WORD_BITS
}

/// The occupancy or mark bitmap for one heap page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Bitmap<const CAPACITY: usize, const WORDS: usize>([u64; WORDS]);

impl<const CAPACITY: usize, const WORDS: usize> Bitmap<CAPACITY, WORDS> {
    /// Create one empty bitmap.
    pub const fn new() -> Self {
        assert!(WORDS == bitmap_word_count(CAPACITY));

        Self([0; WORDS])
    }

    /// Report whether one slot offset is set.
    #[inline]
    pub fn contains(self, offset: usize) -> bool {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.0[word_index] & mask != 0
    }

    /// Set one slot offset.
    #[inline]
    pub fn set(&mut self, offset: usize) {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = 1_u64 << bit_offset;

        self.0[word_index] |= mask;
    }

    /// Clear one slot offset.
    #[inline]
    pub fn clear(&mut self, offset: usize) {
        let word_index = offset / PAGE_BITMAP_WORD_BITS;
        let bit_offset = offset % PAGE_BITMAP_WORD_BITS;
        let mask = !(1_u64 << bit_offset);

        self.0[word_index] &= mask;
    }

    /// Clear all bits.
    #[inline]
    pub fn clear_all(&mut self) {
        self.0.fill(0);
    }

    /// Count the number of set bits.
    #[inline]
    pub fn count_ones(self) -> usize {
        self.0.iter().map(|word| word.count_ones() as usize).sum()
    }
}

impl<const CAPACITY: usize, const WORDS: usize> Default for Bitmap<CAPACITY, WORDS> {
    /// Create one empty bitmap.
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize, const WORDS: usize> Serialize for Bitmap<CAPACITY, WORDS> {
    /// Serialize this bitmap as one fixed-width tuple of words.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple(WORDS)?;

        // words
        for word in self.0 {
            tuple.serialize_element(&word)?;
        }

        tuple.end()
    }
}

/// A serde visitor for one fixed-size page bitmap.
struct BitmapVisitor<const CAPACITY: usize, const WORDS: usize>;

impl<'de, const CAPACITY: usize, const WORDS: usize> Visitor<'de>
    for BitmapVisitor<CAPACITY, WORDS>
{
    /// The bitmap produced by this visitor.
    type Value = Bitmap<CAPACITY, WORDS>;

    /// Describe the bitmap expected by this visitor.
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a bitmap with {WORDS} words")
    }

    /// Visit one sequence of bitmap words.
    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut words = [0_u64; WORDS];

        // words
        for (index, word) in words.iter_mut().enumerate() {
            *word = seq
                .next_element()?
                .ok_or_else(|| A::Error::invalid_length(index, &self))?;
        }

        Ok(Bitmap(words))
    }
}

impl<'de, const CAPACITY: usize, const WORDS: usize> Deserialize<'de> for Bitmap<CAPACITY, WORDS> {
    /// Deserialize one fixed-width bitmap tuple.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(WORDS, BitmapVisitor::<CAPACITY, WORDS>)
    }
}
