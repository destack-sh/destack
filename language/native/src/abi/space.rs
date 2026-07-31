use serde::{Deserialize, Serialize};

/// Runtime ownership domain passed through the native ABI.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// Worker-local heap storage.
    Local = 0,
    /// Runtime-shared heap storage.
    Shared = 1,
}

/// Native projection of immutable constant memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstantSpace {
    /// The first byte in the constant space.
    pub bytes: *const u8,
    /// The constant space byte count.
    pub byte_len: usize,
}

/// Native projection of mutable static memory.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticSpace {
    /// The first byte in the static space.
    pub bytes: *mut u8,
    /// The static space byte count.
    pub byte_len: usize,
}
