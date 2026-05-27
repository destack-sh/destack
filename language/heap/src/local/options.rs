use serde::{Deserialize, Serialize};

use crate::allocator::{
    Allocator, DEFAULT_ALLOCATOR_CHUNK_BYTES, DEFAULT_PAGE_BYTES, SizeClassTable,
};
use crate::{
    AllocationClass, DEFAULT_MAX_MANAGED_YOUNG_ALLOCATION_BYTES,
    DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES, DEFAULT_SMALL_BYTES, DEFAULT_SPACE_BYTES,
    DEFAULT_YOUNG_BYTES, GcOptions, HeapError, allocation_class, validate_allocator_chunk_bytes,
    validate_page_bytes, validate_size_class_alignment, validate_small_span_bytes,
    validate_space_bytes,
};

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
    /// Resolve one managed allocation class for this heap shape.
    #[inline(always)]
    pub fn allocation_class(
        &self,
        byte_len: usize,
        alignment: usize,
        is_noscan: bool,
    ) -> AllocationClass {
        allocation_class(
            byte_len,
            alignment,
            is_noscan,
            &self.size_classes,
            self.page_bytes,
            self.heap_small_bytes,
        )
    }

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

    /// Validate the local heap allocation shape.
    fn validate_allocation_shape(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_bytes(self.page_bytes)?;
        validate_allocator_chunk_bytes(self.page_bytes, self.allocator_chunk_bytes)?;
        validate_space_bytes(self.page_bytes, self.heap_space_bytes)?;
        validate_space_bytes(self.page_bytes, self.raw_space_bytes)?;

        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_bytes(self.heap_small_bytes, &self.size_classes)?;
        validate_small_span_bytes(self.raw_small_bytes, &self.size_classes)?;

        Ok(())
    }

    /// Validate these options for one heap.
    pub fn validate_local(&self) -> Result<(), HeapError> {
        self.validate_allocation_shape()?;

        // reject contradictory young-space policy
        if self.heap_young_bytes != 0
            && self.max_heap_young_allocation_bytes > self.heap_young_bytes
        {
            return Err(HeapError::HeapYoungThresholdExceedsCapacity {
                threshold: self.max_heap_young_allocation_bytes,
                capacity: self.heap_young_bytes,
            });
        }

        // keep young side metadata compact and directly indexed
        let max_young_bytes = u32::MAX as usize;
        if self.heap_young_bytes > max_young_bytes {
            return Err(HeapError::HeapYoungCapacityTooLarge {
                capacity: self.heap_young_bytes,
                max: max_young_bytes,
            });
        }

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

    /// Reject invalid GC trigger percentages at heap construction.
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
