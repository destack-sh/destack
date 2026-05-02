use serde::{Deserialize, Serialize};

/// Native code address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CodePointer(usize);

impl CodePointer {
    /// Create one code pointer from native address bits.
    pub const fn from_address(address: usize) -> Self {
        Self(address)
    }

    /// Return the native address bits.
    pub const fn address(self) -> usize {
        self.0
    }

    /// Return whether this is a null address.
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// Byte offset inside code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CodeOffset(pub u32);

/// Byte length inside code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CodeLen(pub u32);
