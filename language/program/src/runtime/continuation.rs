use crate::{FrameImage, StackImage};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

/// Durable continuation image captured at one managed safepoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ContinuationImage {
    /// The captured stack bytes.
    pub stack: StackImage,
    /// The captured frames from outermost to innermost.
    pub frames: Vec<FrameImage>,
}
