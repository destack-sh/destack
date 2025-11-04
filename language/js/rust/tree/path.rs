use std::fmt::Debug;

use crate::StringId;
use dyst_container::SmallVec;

#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub segments: SmallVec<StringId, 3>,
}
