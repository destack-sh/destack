use serde::{Deserialize, Serialize};

/// One frame state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameStateId(pub u32);
