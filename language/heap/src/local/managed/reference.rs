use serde::{Deserialize, Serialize};

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_OFFSET_SHIFT: u64 = 32;

/// Reference to one local managed allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManagedReference(pub(crate) u64);

impl ManagedReference {
    /// The null managed reference.
    pub const NULL: Self = Self(0);

    /// Create a managed reference from one stable id.
    #[inline]
    pub fn new(id: u32) -> Self {
        Self::with_byte_offset(id, 0)
    }

    /// Report whether this reference is null.
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

    /// Restore one managed reference from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the stored byte offset.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        (self.0 >> POINTER_OFFSET_SHIFT) as usize
    }

    /// Create a managed reference with one byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        let base = id as u64;
        let offset = (byte_offset as u64) << POINTER_OFFSET_SHIFT;

        Self(base | offset)
    }
}
