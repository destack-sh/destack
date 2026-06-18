use serde::{Deserialize, Serialize};

use crate::StaticId;

const STATIC_ADDRESS_OFFSET_BITS: u32 = 32;
const STATIC_ADDRESS_OFFSET_MASK: u64 = u32::MAX as u64;

/// Stable address inside one static region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StaticAddress(u64);

impl StaticAddress {
    /// Create a static address.
    #[inline]
    pub const fn new(id: StaticId, byte_offset: u32) -> Self {
        let id = (id.0 as u64) << STATIC_ADDRESS_OFFSET_BITS;
        let byte_offset = byte_offset as u64;

        Self(id | byte_offset)
    }

    /// Create a static address from raw cell bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the static region id.
    #[inline]
    pub const fn id(self) -> StaticId {
        StaticId((self.0 >> STATIC_ADDRESS_OFFSET_BITS) as u32)
    }

    /// Return the byte offset inside the static region.
    #[inline]
    pub const fn byte_offset(self) -> usize {
        (self.0 & STATIC_ADDRESS_OFFSET_MASK) as usize
    }

    /// Return raw cell bits.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Add one byte offset to this static address.
    #[inline]
    pub fn add_bytes(self, byte_offset: usize) -> Option<Self> {
        let byte_offset = self.byte_offset().checked_add(byte_offset)?;
        let byte_offset = u32::try_from(byte_offset).ok()?;

        Some(Self::new(self.id(), byte_offset))
    }
}
