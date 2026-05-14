use std::fmt::Debug;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::StringId;

/// A Path is static path to a named declaration in a namespace.
/// In the case of a Use declaration, the Path excludes the items.
///
/// Examples:
/// ```
/// foo
/// foobar
/// foo.bar.baz.qux
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    /// The path segments.
    pub segments: SmallVec<[StringId; 3]>,
}

impl Path {
    /// Return the last path segment.
    #[inline]
    pub fn last_segment(&self) -> Option<StringId> {
        self.segments.last().copied()
    }
}
