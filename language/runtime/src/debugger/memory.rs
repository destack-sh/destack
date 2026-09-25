use serde::{Deserialize, Serialize};
use tspp_memory::MemoryRange;
use tspp_serde::Reflect;

use crate::runtime::RuntimeId;
use crate::worker::WorkerId;
use crate::world::Moment;

/// One World's mapped memory Regions at one Moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct MemoryMap {
    /// Moment represented by the memory map.
    pub moment: Moment,
    /// Mapped Regions in logical address order.
    pub regions: Vec<Region>,
}

/// One mapped World memory Region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Region {
    /// Logical memory range.
    pub range: MemoryRange,
    /// Region purpose.
    pub kind: RegionKind,
    /// Owning Runtime when the Region is Runtime-local.
    pub runtime_id: Option<RuntimeId>,
    /// Owning Worker when the Region is Worker-local.
    pub worker_id: Option<WorkerId>,
}

/// Logical purpose of one mapped World memory Region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum RegionKind {
    /// Immutable Program bytes.
    Program,
    /// Immutable Program constant bytes.
    Constant,
    /// Runtime shared static storage.
    SharedStatic,
    /// Worker local static storage.
    LocalStatic,
    /// Runtime shared heap storage.
    SharedHeap,
    /// Worker local heap storage.
    Heap,
    /// Fiber stack storage.
    Stack,
}
