use serde::{Deserialize, Serialize};

use destack_mir::TraceId;

use crate::allocator::{
    Allocator, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
    SizeClassTable,
};
use crate::{
    AllocationClass, AllocationShape, AllocationSite, DEFAULT_ADDRESS_SPACE_SIZE_BYTES,
    DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    DEFAULT_SMALL_SIZE_BYTES, DEFAULT_YOUNG_SIZE_BYTES, GcOptions, HeapConfigurationError,
    HeapError, allocation_class, validate_address_space_size_bytes,
    validate_allocator_chunk_size_bytes, validate_page_size_bytes, validate_size_class_alignment,
    validate_small_span_size_bytes,
};

/// The configuration for one heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The byte size for heap young space.
    pub heap_young_size_bytes: usize,
    /// The maximum payload size routed to heap young space.
    pub max_heap_young_allocation_size_bytes: usize,
    /// The byte size for heap small-block spans.
    pub heap_small_size_bytes: usize,
    /// The virtual byte capacity for heap storage.
    pub address_space_size_bytes: usize,
    /// The byte size for allocator pages.
    pub page_size_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_size_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
}

impl HeapOptions {
    /// Resolve one heap allocation site for this allocation shape.
    #[inline(always)]
    pub fn allocation_site_for_shape(&self, shape: AllocationShape<'_>) -> AllocationSite {
        let class = self.allocation_class_for_shape(shape);

        AllocationSite::new(shape, class)
    }

    /// Resolve one heap allocation class for this allocation shape.
    #[inline(always)]
    fn allocation_class_for_shape(&self, shape: AllocationShape<'_>) -> AllocationClass {
        if shape.trace_map.has_tagged_reference() {
            return AllocationClass::Large;
        }

        self.allocation_class(
            shape.byte_len,
            shape.alignment,
            shape.trace_id,
            shape.is_noscan,
        )
    }

    /// Resolve one heap allocation class for allocation parameters.
    #[inline(always)]
    pub(crate) fn allocation_class(
        &self,
        byte_len: usize,
        alignment: usize,
        trace_id: Option<TraceId>,
        is_noscan: bool,
    ) -> AllocationClass {
        allocation_class(
            byte_len,
            alignment,
            trace_id,
            is_noscan,
            &self.size_classes,
            self.page_size_bytes,
            self.heap_small_size_bytes,
        )
    }

    /// Build the default option set for one heap.
    pub fn local() -> Self {
        Self {
            gc: GcOptions::local(),
            size_classes: SizeClassTable::default(),
            heap_young_size_bytes: DEFAULT_YOUNG_SIZE_BYTES,
            max_heap_young_allocation_size_bytes: DEFAULT_MAX_HEAP_YOUNG_ALLOCATION_SIZE_BYTES,
            heap_small_size_bytes: DEFAULT_SMALL_SIZE_BYTES,
            address_space_size_bytes: DEFAULT_ADDRESS_SPACE_SIZE_BYTES,
            page_size_bytes: DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
            allocator_chunk_size_bytes: DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }

    /// Validate the local heap allocation shape.
    fn validate_allocation_shape(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_size_bytes(self.page_size_bytes)?;
        validate_allocator_chunk_size_bytes(self.page_size_bytes, self.allocator_chunk_size_bytes)?;
        validate_address_space_size_bytes(self.page_size_bytes, self.address_space_size_bytes)?;

        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_size_bytes(self.heap_small_size_bytes, &self.size_classes)?;

        Ok(())
    }

    /// Validate these options for one heap.
    pub fn validate_local(&self) -> Result<(), HeapError> {
        self.validate_allocation_shape()?;

        // reject contradictory young space policy
        if self.heap_young_size_bytes != 0
            && self.max_heap_young_allocation_size_bytes > self.heap_young_size_bytes
        {
            return Err(HeapError::configuration(
                HeapConfigurationError::YoungThresholdExceedsCapacity {
                    threshold: self.max_heap_young_allocation_size_bytes,
                    capacity: self.heap_young_size_bytes,
                },
            ));
        }

        // keep young side metadata compact and directly indexed
        let max_young_size_bytes = u32::MAX as usize;
        if self.heap_young_size_bytes > max_young_size_bytes {
            return Err(HeapError::configuration(
                HeapConfigurationError::YoungCapacityTooLarge {
                    capacity: self.heap_young_size_bytes,
                    max: max_young_size_bytes,
                },
            ));
        }

        Ok(())
    }

    /// Validate that one explicit allocator matches these heap options.
    pub(crate) fn validate_allocator(&self, allocator: &Allocator) -> Result<(), HeapError> {
        if allocator.page_size_bytes() != self.page_size_bytes {
            return Err(HeapError::configuration(
                HeapConfigurationError::AllocatorPageSizeMismatch {
                    option_page_size_bytes: self.page_size_bytes,
                    allocator_page_size_bytes: allocator.page_size_bytes(),
                },
            ));
        }

        if allocator.chunk_size_bytes() != self.allocator_chunk_size_bytes {
            return Err(HeapError::configuration(
                HeapConfigurationError::AllocatorChunkSizeMismatch {
                    option_chunk_size_bytes: self.allocator_chunk_size_bytes,
                    allocator_chunk_size_bytes: allocator.chunk_size_bytes(),
                },
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        Allocator, GcOptions, Heap, HeapConfigurationError, HeapError, HeapLimits, HeapOptions,
        SizeClassTable,
    };

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
            Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::configuration(HeapConfigurationError::InvalidGcTriggerPercent {
                percent: 101,
            })
        );
    }

    /// Reject contradictory heap young space admission policy.
    #[test]
    fn test_heap_rejects_young_threshold_above_capacity() {
        let options = HeapOptions {
            heap_young_size_bytes: 1024,
            max_heap_young_allocation_size_bytes: 2048,
            ..HeapOptions::local()
        };

        let allocator = Arc::new(
            Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::configuration(HeapConfigurationError::YoungThresholdExceedsCapacity {
                threshold: 2048,
                capacity: 1024
            })
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
            Allocator::try_new(options.page_size_bytes, options.allocator_chunk_size_bytes)
                .expect("allocator should build"),
        );
        let error =
            Heap::with_allocator_limits_and_options(allocator, HeapLimits::default(), options)
                .expect_err("invalid heap options should fail loudly");

        assert_eq!(
            error,
            HeapError::configuration(HeapConfigurationError::MisalignedSizeClass {
                alignment_bytes: 16,
                class_bytes: 24,
            })
        );
    }
}
