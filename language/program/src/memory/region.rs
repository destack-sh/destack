use serde::{Deserialize, Serialize};

use crate::TypeId;

/// One static region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaticId(pub u32);

impl StaticId {
    /// Create one static id.
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// One typed region inside static memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticRegion {
    /// The region id.
    pub id: StaticId,
    /// The region byte offset.
    pub offset: usize,
    /// The region byte length.
    pub byte_len: usize,
    /// The region value type.
    pub ty: TypeId,
    /// Whether this region allows stores.
    pub is_mutable: bool,
}
