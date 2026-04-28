use serde::{Deserialize, Serialize};

/// Pointer to one shared raw allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedRawPointer(pub(crate) usize);

impl SharedRawPointer {
    /// The null shared raw pointer.
    pub const NULL: Self = Self(0);

    /// Create a shared raw pointer from one space offset.
    #[inline]
    pub const fn new(offset: usize) -> Self {
        Self(offset)
    }

    /// Report whether this pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Return the raw packed bits.
    #[inline]
    pub const fn bits(self) -> usize {
        self.0
    }

    /// Restore one shared raw pointer from raw bits.
    #[inline]
    pub const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    /// Return the space offset.
    #[inline]
    pub const fn offset(self) -> usize {
        self.0
    }

    /// Return one pointer advanced by the given byte offset.
    #[inline]
    pub fn add_bytes(self, byte_len: usize) -> Self {
        let offset = self.offset() + byte_len;

        Self::new(offset)
    }
}
