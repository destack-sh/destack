use serde::{Deserialize, Serialize};

use crate::allocator::{
    Allocator, DEFAULT_ALLOCATOR_CHUNK_BYTES, DEFAULT_PAGE_BYTES, SizeClassTable,
};
use crate::{
    AllocationClass, DEFAULT_SHARED_SMALL_BYTES, DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
    DEFAULT_SPACE_BYTES, GcOptions, HeapError, allocation_class, validate_allocator_chunk_bytes,
    validate_page_bytes, validate_size_class_alignment, validate_small_span_bytes,
    validate_space_bytes,
};

/// The configuration for one shared heap instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedHeapOptions {
    /// The collector configuration.
    pub gc: GcOptions,
    /// The configured small-allocation class table.
    pub size_classes: SizeClassTable,
    /// The byte size for shared heap small-allocation spans.
    pub heap_small_bytes: usize,
    /// The virtual byte capacity for shared heap space.
    pub heap_space_bytes: usize,
    /// The virtual byte capacity for shared raw space.
    pub raw_space_bytes: usize,
    /// The byte size for allocator pages.
    pub page_bytes: usize,
    /// The byte size for one physical allocator chunk.
    pub allocator_chunk_bytes: usize,
    /// The required alignment for configured small-allocation classes.
    pub small_allocation_alignment_bytes: usize,
}

impl Default for SharedHeapOptions {
    fn default() -> Self {
        Self {
            gc: GcOptions::shared(),
            size_classes: SizeClassTable::default(),
            heap_small_bytes: DEFAULT_SHARED_SMALL_BYTES,
            heap_space_bytes: DEFAULT_SPACE_BYTES,
            raw_space_bytes: DEFAULT_SPACE_BYTES,
            page_bytes: DEFAULT_PAGE_BYTES,
            allocator_chunk_bytes: DEFAULT_ALLOCATOR_CHUNK_BYTES,
            small_allocation_alignment_bytes: DEFAULT_SMALL_ALLOCATION_ALIGNMENT_BYTES,
        }
    }
}

impl SharedHeapOptions {
    /// Resolve one managed allocation class for this shared heap shape.
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

    /// Validate these options for one shared heap.
    pub fn validate(&self) -> Result<(), HeapError> {
        self.gc.validate()?;
        validate_page_bytes(self.page_bytes)?;
        validate_allocator_chunk_bytes(self.page_bytes, self.allocator_chunk_bytes)?;
        validate_space_bytes(self.page_bytes, self.heap_space_bytes)?;
        validate_space_bytes(self.page_bytes, self.raw_space_bytes)?;

        validate_size_class_alignment(&self.size_classes, self.small_allocation_alignment_bytes)?;
        validate_small_span_bytes(self.heap_small_bytes, &self.size_classes)?;

        Ok(())
    }

    /// Validate that one explicit allocator matches these shared heap options.
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
