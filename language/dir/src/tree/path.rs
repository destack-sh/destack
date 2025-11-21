use dyst_source::SmallVec;

use crate::StringId;

/// Path to something.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// The segments of the path.
    pub segments: SmallVec<StringId, 3>,
}
