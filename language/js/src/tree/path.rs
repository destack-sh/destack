use std::fmt::Debug;

use crate::StringId;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use tspp_serde::Reflect;

/// A Path is a sequence of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Path {
    /// The segments of the path.
    pub segments: SmallVec<[StringId; 3]>,
}
