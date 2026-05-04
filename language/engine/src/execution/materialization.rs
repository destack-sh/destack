use serde::{Deserialize, Serialize};

use crate::{FrameLayoutId, FrameSlotId, FrameStateId, SafepointId, SlotSource, ValueSource};

/// One materialization id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterializationId(pub u32);

/// Recipe for reconstructing one frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameMaterialization {
    /// The reconstructed frame layout.
    pub frame_layout: FrameLayoutId,
    /// The reconstructed frame state.
    pub frame_state: FrameStateId,
    /// Sources for reconstructed slots.
    pub sources: Vec<SlotSource>,
}

impl FrameMaterialization {
    /// Return each source frame slot once.
    pub fn copied_slots(&self) -> impl Iterator<Item = FrameSlotId> + '_ {
        self.sources
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                let ValueSource::Slot(slot) = &value.source else {
                    return None;
                };

                let is_duplicate = self.sources[..index].iter().any(
                    |prior| matches!(&prior.source, ValueSource::Slot(prior) if prior == slot),
                );
                (!is_duplicate).then_some(*slot)
            })
    }
}

/// Recipe for reconstructing one safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Materialization {
    /// The materialization id.
    pub id: MaterializationId,
    /// The owning safepoint.
    pub safepoint: SafepointId,
    /// The reconstructed frames from outermost to innermost.
    pub frames: Vec<FrameMaterialization>,
}
