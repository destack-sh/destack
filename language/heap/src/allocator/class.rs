use serde::{Deserialize, Serialize};

use crate::HeapError;

/// One span-backed allocation path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanAllocationPath {
    /// One small-space allocation.
    Small {
        /// The resolved size-class index.
        class_index: usize,
        /// The resolved slot width.
        size_class: usize,
        /// The mapped-byte delta for this path.
        mapped_delta: i64,
    },
    /// One large-space allocation.
    Large {
        /// The mapped-byte delta for this path.
        mapped_delta: i64,
    },
}

impl SpanAllocationPath {
    /// Return the mapped-byte delta for this allocation path.
    pub(crate) fn mapped_delta(self) -> i64 {
        match self {
            Self::Small { mapped_delta, .. } | Self::Large { mapped_delta } => mapped_delta,
        }
    }
}

/// One policy for generating size classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SizeClassPolicy {
    /// The smallest generated size class in bytes.
    pub min_bytes: usize,
    /// The largest generated size class in bytes.
    pub max_bytes: usize,
    /// The required alignment for every generated size class.
    pub alignment_bytes: usize,
    /// The maximum internal waste numerator between adjacent classes.
    pub max_waste_numerator: usize,
    /// The maximum internal waste denominator between adjacent classes.
    pub max_waste_denominator: usize,
}

impl Default for SizeClassPolicy {
    fn default() -> Self {
        Self {
            min_bytes: 16,
            max_bytes: 4 * 1024,
            alignment_bytes: 8,
            max_waste_numerator: 1,
            max_waste_denominator: 4,
        }
    }
}

impl SizeClassPolicy {
    /// Generate the size-class table described by this policy.
    pub fn size_classes(self) -> Result<SizeClassTable, HeapError> {
        self.validate()?;

        SizeClassTable::new(self.generate_classes())
    }

    /// Generate raw classes from a validated policy.
    fn generate_classes(self) -> Vec<usize> {
        let mut classes = Vec::new();
        let mut bytes = align_up(self.min_bytes, self.alignment_bytes);
        let max_bytes = align_up(self.max_bytes, self.alignment_bytes);

        classes.push(bytes);

        while bytes < max_bytes {
            let previous_boundary = bytes.saturating_add(1);
            let waste_step = previous_boundary.saturating_mul(self.max_waste_numerator)
                / self.max_waste_denominator;
            let step = waste_step.max(self.alignment_bytes);
            let step = align_down(step, self.alignment_bytes).max(self.alignment_bytes);
            let next_bytes = align_up(bytes.saturating_add(step), self.alignment_bytes);

            bytes = next_bytes.min(max_bytes);
            classes.push(bytes);
        }

        classes
    }

    /// Validate this small object policy.
    fn validate(self) -> Result<(), HeapError> {
        if self.min_bytes == 0 || self.max_bytes == 0 {
            return Err(HeapError::ZeroSizeClass);
        }

        if self.min_bytes > self.max_bytes {
            return Err(HeapError::InvalidSizeClassPolicyRange {
                min_bytes: self.min_bytes,
                max_bytes: self.max_bytes,
            });
        }

        if self.alignment_bytes == 0 || !self.alignment_bytes.is_power_of_two() {
            return Err(HeapError::InvalidSizeClassPolicyAlignment {
                alignment_bytes: self.alignment_bytes,
            });
        }

        if self.max_waste_numerator == 0 || self.max_waste_denominator == 0 {
            return Err(HeapError::InvalidSizeClassPolicyWaste {
                numerator: self.max_waste_numerator,
                denominator: self.max_waste_denominator,
            });
        }

        Ok(())
    }
}

/// One fixed-size small allocation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SizeClass {
    /// The slot payload size in bytes.
    pub bytes: usize,
}

impl SizeClass {
    /// Create one size class.
    pub const fn new(bytes: usize) -> Self {
        Self { bytes }
    }
}

/// One canonical size-class table used by the heap.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SizeClassTable {
    /// The ordered size classes in bytes.
    pub classes: Vec<SizeClass>,
}

impl SizeClassTable {
    /// Create one validated size-class table.
    pub fn new(classes: impl IntoIterator<Item = usize>) -> Result<Self, HeapError> {
        let classes = classes.into_iter().collect::<Vec<_>>();

        Self::validate(&classes)?;

        Ok(Self {
            classes: classes.into_iter().map(SizeClass::new).collect(),
        })
    }

    /// Return the default policy-generated size-class table.
    pub fn default_table() -> Self {
        let policy = SizeClassPolicy::default();
        debug_assert!(policy.size_classes().is_ok());

        Self {
            classes: policy
                .generate_classes()
                .into_iter()
                .map(SizeClass::new)
                .collect(),
        }
    }

    /// Return the largest span-allocated payload size in bytes.
    pub fn max_small_allocation_bytes(&self) -> usize {
        let Some(class) = self.classes.last() else {
            panic!("size class table should not be empty");
        };

        class.bytes
    }

    /// Return the smallest span-allocated payload size in bytes.
    pub fn min_small_allocation_bytes(&self) -> usize {
        let Some(class) = self.classes.first() else {
            panic!("size class table should not be empty");
        };

        class.bytes
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

    /// Return one span-backed allocation path for the requested payload size.
    pub(crate) fn span_allocation_path(
        &self,
        byte_len: usize,
        span_bytes: usize,
        has_available_span: impl FnOnce(usize) -> bool,
        large_bytes: impl FnOnce(usize) -> u64,
    ) -> SpanAllocationPath {
        // use one size-classed span when the payload still fits
        if let Some(class_index) = self.class_index_for(byte_len) {
            let size_class = self.classes[class_index].bytes;
            let mapped_delta = if has_available_span(class_index) {
                0
            } else {
                span_bytes as i64
            };

            return SpanAllocationPath::Small {
                class_index,
                size_class,
                mapped_delta,
            };
        }

        // otherwise fall back to the dedicated large-entry path
        SpanAllocationPath::Large {
            mapped_delta: large_bytes(byte_len) as i64,
        }
    }

    /// Validate one raw size-class list.
    fn validate(classes: &[usize]) -> Result<(), HeapError> {
        // reject empty tables
        if classes.is_empty() {
            return Err(HeapError::EmptySizeClassTable);
        }

        // validate each class in order
        let mut previous = 0usize;
        for &bytes in classes {
            // can't have zero
            if bytes == 0 {
                return Err(HeapError::ZeroSizeClass);
            }

            // require strict monotonic growth
            if bytes <= previous {
                return Err(HeapError::NonMonotonicSizeClass { previous, bytes });
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

/// Return the offset rounded up to the nearest alignment boundary.
fn align_up(bytes: usize, alignment_bytes: usize) -> usize {
    bytes.saturating_add(alignment_bytes.saturating_sub(1)) / alignment_bytes * alignment_bytes
}
