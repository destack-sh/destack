use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::FramePoint;

/// One logical frame state in acquisition order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameState {
    /// The logical coordinate represented by this frame state.
    pub point: FramePoint,
    /// The object-local types of live values in acquisition order.
    pub types: Vec<mir::TypeId>,
}

impl FrameState {
    /// Create one logical frame state.
    pub fn new(point: FramePoint, types: impl IntoIterator<Item = mir::TypeId>) -> Self {
        Self {
            point,
            types: types.into_iter().collect(),
        }
    }
}
