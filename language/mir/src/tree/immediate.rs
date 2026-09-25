use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Compact reference to an index list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Reflect)]
pub struct IndexSlice {
    /// Start index in the index buffer.
    pub start: u32,
    /// Number of indices in the slice.
    pub count: u16,
}

impl IndexSlice {
    /// Create a new index slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of indices in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Compact reference to an extent list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Reflect)]
pub struct ExtentSlice {
    /// Start index in the extent buffer.
    pub start: u32,
    /// Number of extents in the slice.
    pub count: u16,
}

impl ExtentSlice {
    /// Create a new extent slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of extents in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}

/// Compact reference to a flag list stored in the MIR tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Reflect)]
pub struct FlagSlice {
    /// Start index in the flag buffer.
    pub start: u32,
    /// Number of flags in the slice.
    pub count: u16,
}

impl FlagSlice {
    /// Create a new flag slice.
    #[inline]
    pub const fn new(start: u32, count: u16) -> Self {
        Self { start, count }
    }

    /// Return whether this slice is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Return the number of flags in this slice.
    #[inline]
    pub const fn len(&self) -> usize {
        self.count as usize
    }
}
