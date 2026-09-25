use serde::{Deserialize, Serialize};
use tspp_mir as mir;
use tspp_serde::Reflect;

use super::FramePoint;

/// One logical frame state in acquisition order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameState {
    /// The logical coordinate represented by this frame state.
    pub point: FramePoint,
    /// The live logical frame slots in acquisition order.
    pub slots: Vec<FrameSlot>,
}

impl FrameState {
    /// Create one logical frame state.
    pub fn new(point: FramePoint, slots: impl IntoIterator<Item = FrameSlot>) -> Self {
        Self {
            point,
            slots: slots.into_iter().collect(),
        }
    }

    /// Iterate the object-local types in acquisition order.
    pub fn types(&self) -> impl ExactSizeIterator<Item = mir::TypeId> + '_ {
        self.slots.iter().map(|slot| slot.ty)
    }
}

/// One live logical value inside a frame state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameSlot {
    /// The logical place containing the value.
    pub place: FramePlace,
    /// The object-local value type.
    pub ty: mir::TypeId,
}

impl FrameSlot {
    /// Create one live logical frame slot.
    pub const fn new(place: FramePlace, ty: mir::TypeId) -> Self {
        Self { place, ty }
    }
}

/// One MIR place retained by an engine frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum FramePlace {
    /// The hidden callable environment.
    Environment,
    /// One addressable MIR local.
    Local(mir::LocalId),
    /// One MIR SSA value.
    Value(mir::Value),
}
