use destack_serde::Schema;
use serde::{Deserialize, Serialize};

use crate::TypeId;

/// One static region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
pub struct StaticId(pub u32);

impl StaticId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for StaticId {
    /// Convert one raw program static id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<StaticId> for u32 {
    /// Convert one program static id into its raw value.
    fn from(id: StaticId) -> Self {
        id.0
    }
}

/// One typed region inside static memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Schema)]
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
