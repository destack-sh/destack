use serde::{Deserialize, Serialize};

use crate::allocator::{
    Allocator, DEFAULT_ALLOCATOR_CHUNK_BYTES, DEFAULT_PAGE_BYTES, SizeClassPolicy, SizeClassTable,
};
use crate::{GcOptions, HeapError};

use super::constants::{
    DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    DEFAULT_SMALL_BYTES, DEFAULT_SPACE_BYTES, DEFAULT_YOUNG_BYTES,
};

/// Constructor policy for resolving heap options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapPolicy {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The small-allocation policy.
    pub small: SizeClassPolicy,
    /// The byte size for heap young space.
    pub heap_young_bytes: usize,
    /// The maximum payload size routed to heap young space.
    pub max_heap_young_allocation_bytes: usize,
    /// The byte size for heap small-allocation spans.
    pub heap_small_bytes: usize,
    /// The byte size for raw small-allocation spans.
    pub raw_small_bytes: usize,
    /// The virtual byte capacity for managed heap space.
    pub heap_space_bytes: usize,
    /// The virtual byte capacity for raw heap space.
    pub raw_space_bytes: usize,
    /// The byte size for allocator pages.
    pub page_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_bytes: usize,
}

impl Default for HeapPolicy {
    fn default() -> Self {
        Self {
            gc: GcOptions::local(),
            small: SizeClassPolicy::default(),
            heap_young_bytes: DEFAULT_YOUNG_BYTES,
            max_heap_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            heap_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            allocator_chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
        }
    }
}

impl HeapPolicy {
    /// Resolve this constructor policy into heap options.
    pub fn resolve(&self) -> Result<HeapOptions, HeapError> {
        let options = HeapOptions {
            gc: self.gc,
            size_classes: self.small.size_classes()?,
            heap_young_bytes: self.heap_young_bytes,
            max_heap_young_allocation_bytes: self.max_heap_young_allocation_bytes,
            heap_small_bytes: self.heap_small_bytes,
            raw_small_bytes: self.raw_small_bytes,
            heap_space_bytes: self.heap_space_bytes,
            raw_space_bytes: self.raw_space_bytes,
            page_bytes: self.page_bytes,
            allocator_chunk_bytes: self.allocator_chunk_bytes,
            small_allocation_alignment_bytes: self.small.alignment_bytes,
        };

        options.validate_local()?;

        Ok(options)
    }
}

/// Constructor policy for resolving shared heap options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapPolicy {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The small-allocation policy.
    pub small: SizeClassPolicy,
    /// The byte size for heap small-allocation spans.
    pub heap_small_bytes: usize,
    /// The byte size for allocator pages.
    pub page_bytes: usize,
    /// The virtual byte capacity for shared heap space.
    pub heap_space_bytes: usize,
    /// The virtual byte capacity for shared raw space.
    pub raw_space_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_bytes: usize,
}

impl Default for SharedHeapPolicy {
    fn default() -> Self {
        Self {
            gc: GcOptions::shared(),
            small: SizeClassPolicy::default(),
            heap_small_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            allocator_chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
        }
    }
}

impl SharedHeapPolicy {
    /// Resolve this constructor policy into heap options.
    pub fn resolve(&self) -> Result<HeapOptions, HeapError> {
        let options = HeapOptions {
            gc: self.gc,
            size_classes: self.small.size_classes()?,
            heap_young_bytes: 0,
            max_heap_young_allocation_bytes: 0,
            heap_small_bytes: self.heap_small_bytes,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: self.heap_space_bytes,
            raw_space_bytes: self.raw_space_bytes,
            page_bytes: self.page_bytes,
            allocator_chunk_bytes: self.allocator_chunk_bytes,
            small_allocation_alignment_bytes: self.small.alignment_bytes,
        };

        options.validate_shared()?;

        Ok(options)
    }
}

/// The configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The byte size for heap young space.
    pub heap_young_bytes: usize,
    /// The maximum payload size routed to heap young space.
    pub max_heap_young_allocation_bytes: usize,
    /// The byte size for heap small-allocation spans.
    pub heap_small_bytes: usize,
    /// The byte size for raw small-allocation spans.
    pub raw_small_bytes: usize,
    /// The virtual byte capacity for managed heap space.
    pub heap_space_bytes: usize,
    /// The virtual byte capacity for raw heap space.
    pub raw_space_bytes: usize,
    /// The byte size for allocator pages.
    pub page_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
}

impl HeapOptions {
    /// Build the default option set for one heap.
    pub fn local() -> Self {
        Self {
            gc: GcOptions::local(),
            size_classes: SizeClassTable::default(),
            heap_young_bytes: DEFAULT_YOUNG_BYTES,
            max_heap_young_allocation_bytes: DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
            heap_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            allocator_chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }

    /// Build the default option set for one shared heap.
    pub fn shared() -> Self {
        Self {
            gc: GcOptions::shared(),
            size_classes: SizeClassTable::default(),
            heap_young_bytes: 0,
            max_heap_young_allocation_bytes: 0,
            heap_small_bytes: DEFAULT_SMALL_BYTES,
            raw_small_bytes: DEFAULT_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            allocator_chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }

    /// Return the number of allocator pages in one small span.
    pub(crate) fn small_span_pages(&self) -> usize {
        self.heap_small_bytes.div_ceil(self.page_bytes).max(1)
    }

    /// Return the minimum number of payload slots in one small span.
    pub(crate) fn minimum_small_span_slots(&self) -> usize {
        self.size_classes
            .min_small_allocation_bytes()
            .map(|min_bytes| self.heap_small_bytes / min_bytes)
            .unwrap_or(1)
            .max(1)
    }

    /// Validate one configured heap page size.
    pub(crate) fn validate_page_bytes(page_bytes: usize) -> Result<usize, HeapError> {
        if page_bytes == 0 || !page_bytes.is_power_of_two() {
            Err(HeapError::InvalidPageBytes { bytes: page_bytes })
        } else {
            Ok(page_bytes)
        }
    }

    /// Validate one configured allocator chunk size.
    pub(crate) fn validate_allocator_chunk_bytes(
        page_bytes: usize,
        allocator_chunk_bytes: usize,
    ) -> Result<usize, HeapError> {
        if allocator_chunk_bytes == 0 {
            return Err(HeapError::InvalidAllocatorChunkBytes {
                bytes: allocator_chunk_bytes,
            });
        }

        if !allocator_chunk_bytes.is_power_of_two() {
            return Err(HeapError::InvalidAllocatorChunkBytes {
                bytes: allocator_chunk_bytes,
            });
        }

        if !allocator_chunk_bytes.is_multiple_of(page_bytes) {
            return Err(HeapError::MisalignedAllocatorChunkBytes {
                page_bytes,
                chunk_bytes: allocator_chunk_bytes,
            });
        }

        Ok(allocator_chunk_bytes)
    }

    /// Validate one configured virtual space size.
    pub(crate) fn validate_space_bytes(
        page_bytes: usize,
        space_bytes: usize,
    ) -> Result<usize, HeapError> {
        if space_bytes == 0 {
            return Err(HeapError::InvalidSpaceBytes { bytes: space_bytes });
        }

        if !space_bytes.is_multiple_of(page_bytes) {
            return Err(HeapError::MisalignedSpaceBytes {
                page_bytes,
                space_bytes,
            });
        }

        Ok(space_bytes)
    }

    /// Validate one configured small-allocation alignment.
    pub(crate) fn validate_small_allocation_alignment_bytes(
        alignment_bytes: usize,
    ) -> Result<usize, HeapError> {
        if alignment_bytes == 0 || !alignment_bytes.is_power_of_two() {
            Err(HeapError::InvalidSmallAllocationAlignmentBytes {
                bytes: alignment_bytes,
            })
        } else {
            Ok(alignment_bytes)
        }
    }

    /// Validate the common page and chunk sizes shared by local and shared heaps.
    fn validate_common(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        Self::validate_page_bytes(self.page_bytes)?;
        Self::validate_allocator_chunk_bytes(self.page_bytes, self.allocator_chunk_bytes)?;
        Self::validate_space_bytes(self.page_bytes, self.heap_space_bytes)?;
        Self::validate_space_bytes(self.page_bytes, self.raw_space_bytes)?;
        Self::validate_small_allocation_alignment_bytes(self.small_allocation_alignment_bytes)?;

        // keep all size classes aligned to the configured small-slot boundary
        for class in &self.size_classes.classes {
            if class.bytes % self.small_allocation_alignment_bytes != 0 {
                return Err(HeapError::MisalignedSizeClass {
                    alignment_bytes: self.small_allocation_alignment_bytes,
                    class_bytes: class.bytes,
                });
            }
        }

        // keep each small span large enough for every configured class
        let max_small_bytes = self
            .size_classes
            .max_small_allocation_bytes()
            .ok_or(HeapError::EmptySizeClassTable)?;
        if self.heap_small_bytes < max_small_bytes {
            return Err(HeapError::SmallSpanTooSmall {
                span_bytes: self.heap_small_bytes,
                class_bytes: max_small_bytes,
            });
        }

        if self.raw_small_bytes < max_small_bytes {
            return Err(HeapError::SmallSpanTooSmall {
                span_bytes: self.raw_small_bytes,
                class_bytes: max_small_bytes,
            });
        }

        Ok(())
    }

    /// Validate these options for one heap.
    pub fn validate_local(&self) -> Result<(), HeapError> {
        self.validate_common()?;

        // reject contradictory young-space policy
        if self.heap_young_bytes != 0
            && self.max_heap_young_allocation_bytes > self.heap_young_bytes
        {
            return Err(HeapError::HeapYoungThresholdExceedsCapacity {
                threshold: self.max_heap_young_allocation_bytes,
                capacity: self.heap_young_bytes,
            });
        }

        Ok(())
    }

    /// Validate these options for one shared heap.
    pub fn validate_shared(&self) -> Result<(), HeapError> {
        self.validate_common()?;

        Ok(())
    }

    /// Validate that one explicit allocator matches these heap options.
    pub(crate) fn validate_allocator(&self, allocator: &Allocator) -> Result<(), HeapError> {
        if allocator.page_bytes() != self.page_bytes {
            return Err(HeapError::AllocatorPageBytesMismatch {
                option_page_bytes: self.page_bytes,
                allocator_page_bytes: allocator.page_bytes(),
            });
        }

        if allocator.chunk_bytes() != self.allocator_chunk_bytes {
            return Err(HeapError::AllocatorChunkBytesMismatch {
                option_chunk_bytes: self.allocator_chunk_bytes,
                allocator_chunk_bytes: allocator.chunk_bytes(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{Allocator, GcOptions, Heap, HeapError, HeapLimits, HeapOptions, SizeClassTable};

    /// Reject unsupported GC trigger percentages at heap construction.
    #[test]
    fn test_heap_rejects_invalid_gc_trigger_percent() {
        let options = HeapOptions {
            gc: GcOptions {
                trigger_percent: 101,
                ..HeapOptions::local().gc
            },
            ..HeapOptions::local()
        };

        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(error, HeapError::InvalidGcTriggerPercent { percent: 101 });
    }

    /// Reject contradictory heap young-space admission policy.
    #[test]
    fn test_heap_rejects_young_threshold_above_capacity() {
        let options = HeapOptions {
            heap_young_bytes: 1024,
            max_heap_young_allocation_bytes: 2048,
            ..HeapOptions::local()
        };

        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::HeapYoungThresholdExceedsCapacity {
                threshold: 2048,
                capacity: 1024,
            }
        );
    }

    /// Reject misaligned size classes under one explicit heap alignment.
    #[test]
    fn test_heap_rejects_misaligned_size_class_table() {
        let options = HeapOptions {
            size_classes: SizeClassTable::new([16, 24, 32])
                .expect("size classes should validate structurally"),
            small_allocation_alignment_bytes: 16,
            ..HeapOptions::local()
        };

        let allocator = Arc::new(
            Allocator::try_new(options.page_bytes, options.allocator_chunk_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::MisalignedSizeClass {
                alignment_bytes: 16,
                class_bytes: 24,
            }
        );
    }
}
