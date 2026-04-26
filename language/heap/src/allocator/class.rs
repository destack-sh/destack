use serde::{Deserialize, Serialize};

use crate::HeapError;

/// Go's small object size classes, excluding class zero.
const GO_SIZE_CLASS_BYTES: [usize; 67] = [
    8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288, 320, 352,
    384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536, 1792, 2048,
    2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192, 9472, 9728,
    10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576, 27264, 28672,
    32768,
];

/// Go's small object span widths in allocator pages, excluding class zero.
const GO_SIZE_CLASS_SPAN_PAGES: [usize; 67] = [
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 2, 1, 2, 1, 2, 1, 3, 2, 3, 1, 3, 2, 3, 4, 5, 6, 1, 7, 6, 5, 4, 3, 5, 7, 2, 9, 7, 5, 8, 3,
    10, 7, 4,
];

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
            min_bytes: Self::DEFAULT_MIN_BYTES,
            max_bytes: Self::DEFAULT_MAX_BYTES,
            alignment_bytes: Self::DEFAULT_ALIGNMENT_BYTES,
            max_waste_numerator: Self::DEFAULT_MAX_WASTE_NUMERATOR,
            max_waste_denominator: Self::DEFAULT_MAX_WASTE_DENOMINATOR,
        }
    }
}

impl SizeClassPolicy {
    /// The default minimum size class in bytes.
    pub const DEFAULT_MIN_BYTES: usize = 8;
    /// The default maximum size class in bytes.
    pub const DEFAULT_MAX_BYTES: usize = 32 * 1024;
    /// The default size class alignment in bytes.
    pub const DEFAULT_ALIGNMENT_BYTES: usize = 8;
    /// The default maximum internal waste numerator.
    pub const DEFAULT_MAX_WASTE_NUMERATOR: usize = 1;
    /// The default maximum internal waste denominator.
    pub const DEFAULT_MAX_WASTE_DENOMINATOR: usize = 4;

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

    /// Validate this small allocation policy.
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
    /// The span width in allocator pages, or zero for the configured fallback span size.
    pub span_pages: usize,
}

impl SizeClass {
    /// Create one size class.
    pub const fn new(bytes: usize) -> Self {
        Self {
            bytes,
            span_pages: 0,
        }
    }

    /// Create one size class with an explicit span page count.
    pub const fn with_span_pages(bytes: usize, span_pages: usize) -> Self {
        Self { bytes, span_pages }
    }

    /// Return the span byte width for this size class.
    pub const fn span_bytes(self, page_bytes: usize, fallback_span_bytes: usize) -> usize {
        if self.span_pages == 0 {
            fallback_span_bytes
        } else {
            self.span_pages * page_bytes
        }
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

    /// Return the default Go size-class table.
    pub fn default_table() -> Self {
        Self {
            classes: GO_SIZE_CLASS_BYTES
                .iter()
                .zip(GO_SIZE_CLASS_SPAN_PAGES)
                .map(|(&bytes, span_pages)| SizeClass::with_span_pages(bytes, span_pages))
                .collect(),
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
