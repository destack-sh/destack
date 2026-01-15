use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::ops::{
    Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

use crate::StringId;

/// Path to something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Path {
    /// The segments of the path.
    pub segments: SmallVec<[StringId; 3]>,
}

impl Path {
    /// Get the first segment of the path.
    pub fn first_segment(&self) -> Option<StringId> {
        self.segments.first().cloned()
    }

    /// Get the last segment of the path.
    pub fn last_segment(&self) -> Option<StringId> {
        self.segments.last().cloned()
    }

    /// Get a subpath by range (like &self[r])
    pub fn slice<R>(&self, range: R) -> Path
    where
        R: Clone + std::slice::SliceIndex<[StringId], Output = [StringId]>,
    {
        Path {
            segments: self.segments[range].into(),
        }
    }
}

impl From<&[StringId]> for Path {
    fn from(segments: &[StringId]) -> Self {
        Path {
            segments: segments.into(),
        }
    }
}

impl Index<usize> for Path {
    type Output = StringId;
    fn index(&self, index: usize) -> &Self::Output {
        &self.segments[index]
    }
}

impl IndexMut<usize> for Path {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.segments[index]
    }
}

impl Index<Range<usize>> for Path {
    type Output = [StringId];
    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.segments[index]
    }
}

impl Index<RangeTo<usize>> for Path {
    type Output = [StringId];
    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.segments[index]
    }
}

impl Index<RangeFrom<usize>> for Path {
    type Output = [StringId];
    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.segments[index]
    }
}

impl Index<RangeFull> for Path {
    type Output = [StringId];
    fn index(&self, index: RangeFull) -> &Self::Output {
        &self.segments[index]
    }
}

impl Index<RangeInclusive<usize>> for Path {
    type Output = [StringId];
    fn index(&self, index: RangeInclusive<usize>) -> &Self::Output {
        &self.segments[index]
    }
}

impl Index<RangeToInclusive<usize>> for Path {
    type Output = [StringId];
    fn index(&self, index: RangeToInclusive<usize>) -> &Self::Output {
        &self.segments[index]
    }
}
