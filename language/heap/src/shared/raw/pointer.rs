use serde::{Deserialize, Serialize};

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_OFFSET_SHIFT: u64 = 32;

/// Pointer to one shared raw allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedRawPointer(pub(crate) u64);

impl SharedRawPointer {
    /// The null shared raw pointer.
    pub const NULL: Self = Self(0);

    /// Create a shared raw pointer from one stable id.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self::with_byte_offset(id, 0)
    }

    /// Report whether this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.id() == 0
    }

    /// Return the stable allocation id.
    #[inline]
    pub fn id(&self) -> u32 {
        (self.0 & POINTER_BASE_MASK) as u32
    }

    /// Return the raw packed bits.
    #[inline]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Restore one shared raw pointer from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the stored byte offset.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        (self.0 >> POINTER_OFFSET_SHIFT) as usize
    }

    /// Create a shared raw pointer with one byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        let base = id as u64;
        let offset = (byte_offset as u64) << POINTER_OFFSET_SHIFT;

        Self(base | offset)
    }
}
