use serde::{Deserialize, Serialize};

/// Reference to one local heap allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HeapReference(pub(crate) usize);

impl HeapReference {
    /// The null heap reference.
    pub const NULL: Self = Self(0);

    /// The packed byte width of one heap reference.
    pub const BYTE_LEN: usize = std::mem::size_of::<Self>();

    /// Create a heap reference from one raw address.
    #[inline]
    pub const fn new(address: usize) -> Self {
        Self(address)
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

    /// Return one reference advanced by the given byte offset.
    #[inline]
    pub fn add_bytes(self, byte_len: usize) -> Option<Self> {
        let address = self.address().checked_add(byte_len)?;

        Some(Self::new(address))
    }
}
