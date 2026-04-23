use serde::{Deserialize, Serialize};

/// Pointer to one shared raw allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedRawPointer(pub(crate) usize);

impl SharedRawPointer {
    /// The null shared raw pointer.
    pub const NULL: Self = Self(0);

    /// Create a shared raw pointer from one raw address.
    #[inline]
    pub const fn new(address: usize) -> Self {
        Self(address)
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

    /// Return the raw address.
    #[inline]
    pub const fn address(self) -> usize {
        self.0
    }

    /// Return the referenced pointer.
    #[inline]
    pub const fn as_ptr(self) -> *mut u8 {
        self.address() as *mut u8
    }

    /// Return one pointer advanced by the given byte offset.
    #[inline]
    pub fn add_bytes(self, byte_len: usize) -> Option<Self> {
        let address = self.address().checked_add(byte_len)?;

        Some(Self::new(address))
    }
}
