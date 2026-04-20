use serde::{Deserialize, Serialize};

const POINTER_BASE_MASK: u64 = 0xFFFF_FFFF;
const POINTER_OFFSET_SHIFT: u64 = 32;

/// Reference to one shared managed allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedManagedReference(pub(crate) u64);

impl SharedManagedReference {
    /// The null shared managed reference.
    pub const NULL: Self = Self(0);

    /// The packed byte width of one shared managed reference.
    pub const BYTE_LEN: usize = std::mem::size_of::<Self>();

    /// Create a shared managed reference from one stable id.
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

    /// Restore one shared managed reference from raw bits.
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Return the stored byte offset.
    #[inline]
    pub fn byte_offset(&self) -> usize {
        (self.0 >> POINTER_OFFSET_SHIFT) as usize
    }

    /// Create a shared managed reference with one byte offset.
    #[inline]
    pub fn with_byte_offset(id: u32, byte_offset: u32) -> Self {
        let base = id as u64;
        let offset = (byte_offset as u64) << POINTER_OFFSET_SHIFT;

        Self(base | offset)
    }
}
