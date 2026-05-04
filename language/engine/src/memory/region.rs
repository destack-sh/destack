use serde::{Deserialize, Serialize};

use crate::LayoutId;

/// One static region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaticId(pub u32);

/// One typed region inside static memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticRegion {
    /// The region id.
    pub id: StaticId,
    /// The region byte offset.
    pub offset: usize,
    /// The region byte length.
    pub byte_len: usize,
    /// The region value layout.
    pub layout: LayoutId,
    /// Whether this region allows stores.
    pub is_mutable: bool,
}
