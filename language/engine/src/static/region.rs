use serde::{Deserialize, Serialize};

use crate::{StaticId, TypeId};

/// One typed byte region inside static memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticRegion {
    /// The static region id.
    pub id: StaticId,
    /// The byte offset in static bytes.
    pub offset: usize,
    /// The byte width of this region.
    pub byte_len: usize,
    /// The value type stored in this region.
    pub ty: TypeId,
    /// Whether stores through this region are allowed.
    pub is_mutable: bool,
}
