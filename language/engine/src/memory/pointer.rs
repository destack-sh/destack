use serde::{Deserialize, Serialize};

/// Address inside one static value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StaticPointer(usize);

impl StaticPointer {
    /// Create a static pointer from one native address.
    #[inline]
    pub const fn from_address(address: usize) -> Self {
        Self(address)
    }

    /// Create a static pointer from raw address-sized bits.
    #[inline]
    pub const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    /// Return the native address.
    #[inline]
    pub const fn address(self) -> usize {
        self.0
    }

    /// Return the native address bits.
    #[inline]
    pub const fn bits(self) -> usize {
        self.0
    }

    /// Add one byte offset to this address.
    #[inline]
    pub const fn add_bytes(self, byte_offset: usize) -> Self {
        Self::from_address(self.address() + byte_offset)
    }
}
