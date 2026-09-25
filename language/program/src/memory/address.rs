use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use crate::Word;

/// Stable address inside static memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[repr(transparent)]
pub struct GlobalAddress(u64);

impl GlobalAddress {
    /// The fixed byte width of one encoded global address.
    pub const BYTE_LEN: usize = std::mem::size_of::<u64>();
    /// The first offset available to static storage.
    pub const FIRST_OFFSET: usize = 2;
    /// The canonical null global address.
    pub const NULL: Self = Self(Word::NULL.bits());
    /// The canonical undefined global address.
    pub const UNDEFINED: Self = Self(Word::UNDEFINED.bits());

    /// Create a global address from one storage-relative byte offset.
    #[inline]
    pub const fn new(offset: usize) -> Self {
        assert!(
            offset >= Self::FIRST_OFFSET,
            "global address overlaps nullish values"
        );

        Self(offset as u64)
    }

    /// Create a global address from raw word bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the storage-relative byte offset when this address is not nullish.
    #[inline]
    pub const fn offset(self) -> Option<usize> {
        if self.is_nullish() || self.0 > usize::MAX as u64 {
            None
        } else {
            Some(self.0 as usize)
        }
    }

    /// Return raw word bits.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Return whether this is the canonical null address.
    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == Self::NULL.0
    }

    /// Return whether this is the canonical undefined address.
    #[inline]
    pub const fn is_undefined(self) -> bool {
        self.0 == Self::UNDEFINED.0
    }

    /// Return whether this address is null or undefined.
    #[inline]
    pub const fn is_nullish(self) -> bool {
        self.is_null() || self.is_undefined()
    }

    /// Add one byte offset to this global address.
    #[inline]
    pub fn add_bytes(self, byte_offset: usize) -> Option<Self> {
        let offset = self.offset()?.checked_add(byte_offset)?;

        Some(Self::new(offset))
    }
}

impl From<Word> for GlobalAddress {
    /// Decode one global address word.
    fn from(word: Word) -> Self {
        Self::from_bits(word.bits())
    }
}

impl From<GlobalAddress> for Word {
    /// Encode one global address word.
    fn from(address: GlobalAddress) -> Self {
        Self::from_bits(address.bits())
    }
}
