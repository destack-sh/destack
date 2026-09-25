use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

use super::size::{DEFAULT_MAX_SMALL_ALLOCATION_BYTES, DEFAULT_SIZE_CLASS_TABLE_CLASSES};
use crate::{
    HeapConfigurationError, HeapError, SizeClassPolicyError, SizeClassTableError, align_up,
};

/// One policy for generating size classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SizeClassPolicy {
    /// The smallest generated size class in bytes.
    pub min_bytes: usize,
    /// The largest generated size class in bytes.
    pub max_bytes: usize,
    /// The required alignment for every generated size class.
    pub alignment_bytes: usize,
    /// The maximum internal fragmentation numerator between adjacent classes.
    pub max_fragmentation_numerator: usize,
    /// The maximum internal fragmentation denominator between adjacent classes.
    pub max_fragmentation_denominator: usize,
}

impl Default for SizeClassPolicy {
    fn default() -> Self {
        Self {
            min_bytes: Self::DEFAULT_MIN_BYTES,
            max_bytes: Self::DEFAULT_MAX_BYTES,
            alignment_bytes: Self::DEFAULT_ALIGNMENT_BYTES,
            max_fragmentation_numerator: Self::DEFAULT_MAX_FRAGMENTATION_NUMERATOR,
            max_fragmentation_denominator: Self::DEFAULT_MAX_FRAGMENTATION_DENOMINATOR,
        }
    }
}

impl SizeClassPolicy {
    /// The default minimum size class in bytes.
    pub const DEFAULT_MIN_BYTES: usize = 8;
    /// The default maximum size class in bytes.
    pub const DEFAULT_MAX_BYTES: usize = DEFAULT_MAX_SMALL_ALLOCATION_BYTES;
    /// The default size class alignment in bytes.
    pub const DEFAULT_ALIGNMENT_BYTES: usize = 8;
    /// The default maximum internal fragmentation numerator.
    pub const DEFAULT_MAX_FRAGMENTATION_NUMERATOR: usize = 1;
    /// The default maximum internal fragmentation denominator.
    pub const DEFAULT_MAX_FRAGMENTATION_DENOMINATOR: usize = 4;

    /// Generate the custom size-class table described by this policy.
    pub fn size_classes(self) -> Result<SizeClassTable, HeapError> {
        self.validate()?;

        if self == Self::default() {
            return Ok(SizeClassTable::default());
        }

        SizeClassTable::new(self.generate_classes())
    }

    /// Generate raw classes from a validated policy.
    fn generate_classes(self) -> Vec<usize> {
        let mut classes = Vec::new();
        let mut bytes = align_up(self.min_bytes, self.alignment_bytes);
        let max_bytes = align_up(self.max_bytes, self.alignment_bytes);

        classes.push(bytes);

        while bytes < max_bytes {
            let previous_boundary = bytes + 1;
            let fragmentation_step = previous_boundary * self.max_fragmentation_numerator
                / self.max_fragmentation_denominator;
            let step = fragmentation_step.max(self.alignment_bytes);
            let step = align_down(step, self.alignment_bytes).max(self.alignment_bytes);
            let next_bytes = align_up(bytes + step, self.alignment_bytes);

            bytes = next_bytes.min(max_bytes);
            classes.push(bytes);
        }

        classes
    }

    /// Validate this small block policy.
    fn validate(self) -> Result<(), HeapError> {
        if self.min_bytes == 0 || self.max_bytes == 0 {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassTable {
                    reason: SizeClassTableError::ZeroSizeClass,
                },
            ));
        }

        if self.min_bytes > self.max_bytes {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassPolicy {
                    reason: SizeClassPolicyError::InvalidRange {
                        min_bytes: self.min_bytes,
                        max_bytes: self.max_bytes,
                    },
                },
            ));
        }

        if self.alignment_bytes == 0 || !self.alignment_bytes.is_power_of_two() {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassPolicy {
                    reason: SizeClassPolicyError::InvalidAlignment {
                        alignment_bytes: self.alignment_bytes,
                    },
                },
            ));
        }

        if self.max_fragmentation_numerator == 0 || self.max_fragmentation_denominator == 0 {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassPolicy {
                    reason: SizeClassPolicyError::InvalidFragmentation {
                        numerator: self.max_fragmentation_numerator,
                        denominator: self.max_fragmentation_denominator,
                    },
                },
            ));
        }

        Ok(())
    }
}

/// One fixed-size small block class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SizeClass {
    /// The slot payload size in bytes.
    pub bytes: usize,
    /// The span width in bytes, or zero for the default span size.
    pub span_size_bytes: usize,
}

impl SizeClass {
    /// Create one size class.
    pub const fn new(bytes: usize) -> Self {
        Self {
            bytes,
            span_size_bytes: 0,
        }
    }

    /// Create one size class with an explicit span byte width.
    pub const fn with_span_size_bytes(bytes: usize, span_size_bytes: usize) -> Self {
        Self {
            bytes,
            span_size_bytes,
        }
    }

    /// Return the span byte width for this size class.
    pub fn span_size_bytes(self, page_size_bytes: usize, default_span_size_bytes: usize) -> usize {
        let span_size_bytes = if self.span_size_bytes == 0 {
            default_span_size_bytes
        } else {
            self.span_size_bytes
        };
        let span_size_bytes = span_size_bytes.div_ceil(page_size_bytes) * page_size_bytes;
        let minimum_span_size_bytes = self.bytes.div_ceil(page_size_bytes) * page_size_bytes;

        span_size_bytes.max(minimum_span_size_bytes)
    }
}

/// One canonical size-class table used by the heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct SizeClassTable {
    /// The ordered size classes in bytes.
    pub classes: Arc<[SizeClass]>,
}

impl SizeClassTable {
    /// Create one validated size-class table.
    pub fn new(classes: impl IntoIterator<Item = usize>) -> Result<Self, HeapError> {
        let classes = classes.into_iter().collect::<Vec<_>>();

        Self::validate(&classes)?;

        Ok(Self {
            classes: classes
                .into_iter()
                .map(SizeClass::new)
                .collect::<Vec<_>>()
                .into(),
        })
    }

    /// Return the default size-class table.
    pub fn default_table() -> Self {
        Self {
            classes: DEFAULT_SIZE_CLASS_TABLE_CLASSES.clone(),
        }
    }

    /// Return the largest span-allocated payload size in bytes.
    pub fn max_small_allocation_bytes(&self) -> Option<usize> {
        self.classes.last().map(|class| class.bytes)
    }

    /// Return the smallest span-allocated payload size in bytes.
    pub fn min_small_allocation_bytes(&self) -> Option<usize> {
        self.classes.first().map(|class| class.bytes)
    }

    /// Return one best-fit size class index for the given payload size.
    pub fn class_index_for(&self, bytes: usize) -> Option<usize> {
        // find the first class that can hold the requested payload
        let class_index = self.classes.partition_point(|class| class.bytes < bytes);

        if class_index == self.classes.len() {
            return None;
        }

        Some(class_index)
    }

    /// Return the smallest size class that satisfies size and alignment.
    pub fn class_index_for_layout(&self, byte_len: usize, alignment: usize) -> Option<usize> {
        let alignment = alignment.max(1);
        let minimum_bytes = byte_len.max(alignment);
        let mut class_index = self
            .classes
            .partition_point(|class| class.bytes < minimum_bytes);

        while class_index < self.classes.len() {
            let class = self.classes[class_index];
            if class.bytes.is_multiple_of(alignment) {
                return Some(class_index);
            }

            class_index += 1;
        }

        None
    }

    /// Return the smallest payload length routed to one class.
    pub(crate) fn class_minimum_byte_len(&self, class_index: usize, alignment: usize) -> usize {
        let alignment = alignment.max(1);
        let mut minimum_byte_len = 1;

        for lower_class in &self.classes[..class_index] {
            if lower_class.bytes.is_multiple_of(alignment) {
                minimum_byte_len = lower_class.bytes + 1;
            }
        }

        minimum_byte_len
    }

    /// Validate one raw size-class list.
    fn validate(classes: &[usize]) -> Result<(), HeapError> {
        // reject empty tables
        if classes.is_empty() {
            return Err(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassTable {
                    reason: SizeClassTableError::Empty,
                },
            ));
        }

        // validate each class in order
        let mut previous = 0usize;
        for &bytes in classes {
            // can't have zero
            if bytes == 0 {
                return Err(HeapError::configuration(
                    HeapConfigurationError::InvalidSizeClassTable {
                        reason: SizeClassTableError::ZeroSizeClass,
                    },
                ));
            }

            // require strict monotonic growth
            if bytes <= previous {
                return Err(HeapError::configuration(
                    HeapConfigurationError::InvalidSizeClassTable {
                        reason: SizeClassTableError::NonMonotonic { previous, bytes },
                    },
                ));
            }

            previous = bytes;
        }

        Ok(())
    }
}

impl Default for SizeClassTable {
    fn default() -> Self {
        Self::default_table()
    }
}

/// Return the offset rounded down to the nearest alignment boundary.
fn align_down(bytes: usize, alignment_bytes: usize) -> usize {
    bytes / alignment_bytes * alignment_bytes
}
