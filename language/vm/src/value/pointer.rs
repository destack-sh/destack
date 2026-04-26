use serde::{Deserialize, Serialize};

/// Address or handle for executable function code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FunctionPointer(usize);

impl FunctionPointer {
    /// Create a function pointer from native address-sized bits.
    #[inline]
    pub const fn from_bits(bits: usize) -> Self {
        Self(bits)
    }

    /// Return the address-sized bits.
    #[inline]
    pub const fn bits(self) -> usize {
        self.0
    }

    /// Return the program-local function index.
    #[inline]
    pub const fn function_index(self) -> u32 {
        self.0 as u32
    }
}

/// Address inside one frame-owned stack allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct StackPointer(usize);

impl StackPointer {
    /// Create a stack pointer from one native address.
    #[inline]
    pub const fn from_address(address: usize) -> Self {
        Self(address)
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
    pub fn add_bytes(self, byte_offset: usize) -> Option<Self> {
        self.0.checked_add(byte_offset).map(Self)
    }
}

/// Address inside one frame value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct FramePointer(usize);

impl FramePointer {
    /// Create a frame pointer from one native address.
    #[inline]
    pub const fn from_address(address: usize) -> Self {
        Self(address)
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
    pub fn add_bytes(self, byte_offset: usize) -> Option<Self> {
        self.0.checked_add(byte_offset).map(Self)
    }
}
