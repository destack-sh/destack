use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::TypeId;

/// Dense executable global id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct GlobalId(pub u32);

impl GlobalId {
    /// Return this id as a dense table index.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

impl From<u32> for GlobalId {
    /// Convert one raw program global id.
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<GlobalId> for u32 {
    /// Convert one program global id into its raw value.
    fn from(id: GlobalId) -> Self {
        id.0
    }
}

/// One executable global stored inside static memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct GlobalRegion {
    /// The global id.
    pub global: GlobalId,
    /// The region byte offset.
    pub offset: usize,
    /// The region byte length.
    pub byte_len: usize,
    /// The region value type.
    pub ty: TypeId,
    /// Whether this region allows stores.
    pub is_mutable: bool,
}
