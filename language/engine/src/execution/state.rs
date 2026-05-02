use crate::FrameRegionId;
use serde::{Deserialize, Serialize};

/// One frame state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameStateId(pub u32);

/// One frame entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameEntryId(pub u32);

/// One frame region move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMove {
    /// The source frame region.
    pub source: FrameRegionId,
    /// The destination frame region.
    pub destination: FrameRegionId,
}

/// Recipe for entering one frame state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameEntry {
    /// The entry id.
    pub id: FrameEntryId,
    /// The frame-region moves applied when entering this state.
    pub moves: Vec<FrameMove>,
    /// The received value region.
    pub received_value: Option<FrameRegionId>,
}

/// Frame state at a safepoint or resume point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameState {
    /// The frame state id.
    pub id: FrameStateId,
    /// The frame entry recipe.
    pub entry: Option<FrameEntryId>,
}
