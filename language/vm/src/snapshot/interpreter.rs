use serde::{Deserialize, Serialize};

use crate::snapshot::FrameImage;

/// Immutable interpreter image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterpreterImage {
    /// The captured frame stack.
    pub stack: Vec<FrameImage>,
}
