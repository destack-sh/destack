use serde::{Deserialize, Serialize};

use crate::{HeapError, HeapResult};

/// Reference to one heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HeapReference(pub(crate) usize);

impl HeapReference {
    /// The null heap reference.
    pub const NULL: Self = Self(0);

    /// The packed byte width of one heap reference.
    pub const BYTE_LEN: usize = std::mem::size_of::<Self>();

    /// Create a heap reference from one space offset.
    #[inline]
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }

    /// Report whether this reference is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Return the raw packed bits.
    #[inline]
    pub const fn bits(self) -> usize {
        self.0
    }

    /// Restore one heap reference from raw bits.
    #[inline]
    pub const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    /// Read one heap reference from a native-width byte window.
    #[inline]
    pub fn read_from_bytes(bytes: &[u8]) -> HeapResult<Self> {
        if bytes.len() != Self::BYTE_LEN {
            return Err(HeapError::InvalidReferenceWindowWidth { bytes: bytes.len() });
        }

        let mut raw = [0u8; Self::BYTE_LEN];
        raw.copy_from_slice(bytes);

        Ok(Self::from_bits(usize::from_le_bytes(raw)))
    }

    /// Write this heap reference into a native-width byte window.
    #[inline]
    pub fn write_to_bytes(self, bytes: &mut [u8]) -> HeapResult<()> {
        if bytes.len() != Self::BYTE_LEN {
            return Err(HeapError::InvalidReferenceWindowWidth { bytes: bytes.len() });
        }

        bytes.copy_from_slice(&self.bits().to_le_bytes());

        Ok(())
    }

    /// Return the space offset.
    #[inline]
    pub const fn offset(self) -> usize {
        self.0
    }

    /// Return one reference advanced by the given byte offset.
    #[inline]
    pub fn add_bytes(self, byte_len: usize) -> Self {
        let offset = self.offset() + byte_len;

        Self::new(offset)
    }
}
