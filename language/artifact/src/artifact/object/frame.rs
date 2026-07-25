use destack_mir as mir;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::Point;

/// One logical executable frame state in canonical slot order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FrameState {
    /// The operation represented by this frame state.
    pub point: Point,
    /// The object-local types of the live values.
    pub types: Vec<mir::TypeId>,
}

impl FrameState {
    /// Create one logical frame state.
    pub fn new(point: Point, types: impl IntoIterator<Item = mir::TypeId>) -> Self {
        Self {
            point,
            types: types.into_iter().collect(),
        }
    }
}
