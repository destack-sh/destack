use std::fmt::Debug;

use crate::StringId;
use dyst_source::SmallVec;

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: SmallVec<StringId, 3>,
}
