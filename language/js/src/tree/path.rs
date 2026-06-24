use std::fmt::Debug;

use crate::StringId;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// A Path is a sequence of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Path {
    /// The segments of the path.
    pub segments: SmallVec<[StringId; 3]>,
}
