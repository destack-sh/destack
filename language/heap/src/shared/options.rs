use serde::{Deserialize, Serialize};

use destack_mir::TraceId;

use crate::allocator::{
    Allocator, DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES, DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
    SizeClassTable,
};
use crate::{
    AllocationClass, DEFAULT_SHARED_SMALL_SIZE_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    DEFAULT_SPACE_SIZE_BYTES, GcOptions, HeapConfigurationError, HeapError, allocation_class,
    validate_allocator_chunk_size_bytes, validate_page_size_bytes, validate_size_class_alignment,
    validate_small_span_size_bytes, validate_space_size_bytes,
};

/// The configuration for one shared heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-block class table.
    pub size_classes: SizeClassTable,
    /// The byte size for shared heap small-block spans.
    pub heap_small_size_bytes: usize,
    /// The virtual byte capacity for shared heap space.
    pub heap_space_size_bytes: usize,
    /// The virtual byte capacity for shared raw space.
    pub raw_space_size_bytes: usize,
    /// The byte size for allocator pages.
    pub page_size_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_size_bytes: usize,
    /// The required alignment for configured small-block classes.
    pub small_allocation_alignment_bytes: usize,
}

impl Default for SharedHeapOptions {
    fn default() -> Self {
        Self {
            gc: GcOptions::shared(),
            size_classes: SizeClassTable::default(),
            heap_small_size_bytes: DEFAULT_SHARED_SMALL_SIZE_BYTES,
            heap_space_size_bytes: DEFAULT_SPACE_SIZE_BYTES,
            raw_space_size_bytes: DEFAULT_SPACE_SIZE_BYTES,
            page_size_bytes: DEFAULT_ALLOCATOR_PAGE_SIZE_BYTES,
            allocator_chunk_size_bytes: DEFAULT_ALLOCATOR_CHUNK_SIZE_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }
}

impl SharedHeapOptions {
    /// Resolve one managed block class for this shared heap shape.
    #[inline(always)]
    pub fn allocation_class(
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

    /// Validate these options for one shared heap.
    pub fn validate(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_size_bytes(self.page_size_bytes)?;
        validate_allocator_chunk_size_bytes(self.page_size_bytes, self.allocator_chunk_size_bytes)?;
        validate_space_size_bytes(self.page_size_bytes, self.heap_space_size_bytes)?;
        validate_space_size_bytes(self.page_size_bytes, self.raw_space_size_bytes)?;

        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_size_bytes(self.heap_small_size_bytes, &self.size_classes)?;

        Ok(())
    }

    /// Validate that one explicit allocator matches these shared heap options.
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
