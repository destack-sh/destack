use serde::{Deserialize, Serialize};

use crate::DEFAULT_SIZE_CLASS_BYTES;

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
    pub fn new(classes: impl IntoIterator<Item = usize>) -> Result<Self, SizeClassTableError> {
        let classes = classes.into_iter().collect::<Vec<_>>();

        Self::validate(&classes)?;

        Ok(Self {
            classes: classes.into_iter().map(SizeClass::new).collect(),
        })
    }

    /// Return the built-in default size-class table.
    pub fn default_table() -> Self {
        debug_assert!(Self::validate(DEFAULT_SIZE_CLASS_BYTES).is_ok());

        Self {
            classes: DEFAULT_SIZE_CLASS_BYTES
                .iter()
                .copied()
                .map(SizeClass::new)
                .collect(),
        }
    }

    /// Return the largest span-allocated payload size in bytes.
    pub fn max_small_allocation_bytes(&self) -> usize {
        self.classes
            .last()
            .map(|class| class.bytes)
            .unwrap_or_default()
    }

    /// Return the smallest span-allocated payload size in bytes.
    pub fn min_small_allocation_bytes(&self) -> usize {
        self.classes.first().map(|class| class.bytes).unwrap_or(1)
    }

    /// Return one best-fit size class index for the given payload size.
    pub fn class_index_for(&self, bytes: usize) -> Option<usize> {
        self.classes.iter().position(|class| class.bytes >= bytes)
    }

    /// Validate one raw size-class list.
    fn validate(classes: &[usize]) -> Result<(), SizeClassTableError> {
        // reject empty tables
        if classes.is_empty() {
            return Err(SizeClassTableError::Empty);
        }

        let mut previous = 0usize;

        // validate each class in order
        for &bytes in classes {
            // can't have zero
            if bytes == 0 {
                return Err(SizeClassTableError::ZeroClass);
            }

            // require strict monotonic growth
            if bytes <= previous {
                return Err(SizeClassTableError::NonMonotonic { previous, bytes });
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

/// Validation error for one configured size-class table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SizeClassTableError {
    /// The configured table is empty.
    Empty,
    /// One size class was zero.
    ZeroClass,
    /// The configured classes were not strictly increasing.
    NonMonotonic { previous: usize, bytes: usize },
}
