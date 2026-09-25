use std::fmt::Debug;
use tspp_serde::Reflect;

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};

use crate::StringId;

/// A static path to a named declaration.
/// In the case of a Use declaration, the Path excludes the items.
///
/// Examples:
/// ```
/// foo
/// foobar
/// foo.bar.baz.qux
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Path {
    /// The path segments.
    pub segments: SmallVec<[StringId; 1]>,
}

impl Path {
    /// Create a path from one segment.
    #[inline]
    pub fn from_segment(segment: StringId) -> Self {
        Self {
            segments: smallvec![segment],
        }
    }

    /// Return the last path segment.
    #[inline]
    pub fn last_segment(&self) -> Option<StringId> {
        self.segments.last().copied()
    }
}
