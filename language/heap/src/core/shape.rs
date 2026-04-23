use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{HeapError, SizeClassTable};

/// One homogeneous small-span payload class.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SmallSpanClass {
    /// The slot payload size in bytes.
    pub(crate) size_class: usize,
    /// Whether every slot in this span has no references.
    pub(crate) is_noscan: bool,
}

impl SmallSpanClass {
    /// Return the number of reusable small-span buckets for one size-class table.
    pub(crate) fn bucket_count(size_classes: &SizeClassTable) -> usize {
        size_classes.classes.len().saturating_mul(2)
    }

    /// Return the reusable-span bucket index for this class.
    pub(crate) fn bucket_index(&self, size_classes: &SizeClassTable) -> Result<usize, HeapError> {
        let Some(class_index) = size_classes.class_index_for(self.size_class) else {
            return Err(HeapError::InvalidSizeClass {
                class_bytes: self.size_class,
            });
        };
        let class_bytes = size_classes.classes[class_index].bytes;
        if class_bytes != self.size_class {
            return Err(HeapError::InvalidSizeClass {
                class_bytes: self.size_class,
            });
        }

        Ok(class_index.saturating_mul(2) + self.is_noscan as usize)
    }
}

impl PartialOrd for SmallSpanClass {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SmallSpanClass {
    fn cmp(&self, other: &Self) -> Ordering {
        self.size_class
            .cmp(&other.size_class)
            .then_with(|| self.is_noscan.cmp(&other.is_noscan))
    }
}
