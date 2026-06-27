use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::GlobalId;

/// Stable address inside static memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[repr(transparent)]
pub struct GlobalAddress(u64);

impl GlobalAddress {
    /// The bit width of the byte offset stored in one global address.
    const OFFSET_BITS: u32 = u32::BITS;
    /// The mask for the byte offset stored in one global address.
    const OFFSET_MASK: u64 = u32::MAX as u64;

    /// Create a global address.
    #[inline]
    pub const fn new(global: GlobalId, byte_offset: u32) -> Self {
        let id = (global.0 as u64) << Self::OFFSET_BITS;
        let byte_offset = byte_offset as u64;

        Self(id | byte_offset)
    }

    /// Create a global address from raw cell bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the addressed global id.
    #[inline]
    pub const fn global(self) -> GlobalId {
        GlobalId((self.0 >> Self::OFFSET_BITS) as u32)
    }

    /// Return the byte offset inside the addressed global.
    #[inline]
    pub const fn byte_offset(self) -> usize {
        (self.0 & Self::OFFSET_MASK) as usize
    }

    /// Return raw cell bits.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Add one byte offset to this global address.
    #[inline]
    pub fn add_bytes(self, byte_offset: usize) -> Option<Self> {
        let byte_offset = self.byte_offset().checked_add(byte_offset)?;
        let byte_offset = u32::try_from(byte_offset).ok()?;

        Some(Self::new(self.global(), byte_offset))
    }
}
