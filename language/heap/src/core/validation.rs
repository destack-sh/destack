use crate::{HeapError, SizeClassTable};

/// The largest page or chunk width stored by allocator metadata.
const MAX_ALLOCATOR_WIDTH_BYTES: usize = u32::MAX as usize;

/// Validate one configured heap page size.
pub(crate) fn validate_page_bytes(page_bytes: usize) -> Result<usize, HeapError> {
    if page_bytes == 0 || page_bytes > MAX_ALLOCATOR_WIDTH_BYTES || !page_bytes.is_power_of_two() {
        Err(HeapError::InvalidPageBytes { bytes: page_bytes })
    } else {
        Ok(page_bytes)
    }
}

/// Validate one configured allocator chunk size against one valid page size.
pub(crate) fn validate_allocator_chunk_bytes(
    page_bytes: usize,
    allocator_chunk_bytes: usize,
) -> Result<usize, HeapError> {
    if allocator_chunk_bytes == 0 || allocator_chunk_bytes > MAX_ALLOCATOR_WIDTH_BYTES {
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

/// Validate one configured virtual space size against one valid page size.
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

/// Validate every size class against one slot alignment.
pub(crate) fn validate_size_class_alignment(
    size_classes: &SizeClassTable,
    alignment_bytes: usize,
) -> Result<(), HeapError> {
    validate_small_allocation_alignment_bytes(alignment_bytes)?;

    for class in &size_classes.classes {
        if class.bytes % alignment_bytes != 0 {
            return Err(HeapError::MisalignedSizeClass {
                alignment_bytes,
                class_bytes: class.bytes,
            });
        }
    }

    Ok(())
}

/// Validate one small span against one size-class table.
pub(crate) fn validate_small_span_bytes(
    span_bytes: usize,
    size_classes: &SizeClassTable,
) -> Result<(), HeapError> {
    let max_small_bytes = size_classes
        .max_small_allocation_bytes()
        .ok_or(HeapError::EmptySizeClassTable)?;

    if span_bytes < max_small_bytes {
        Err(HeapError::SmallSpanTooSmall {
            span_bytes,
            class_bytes: max_small_bytes,
        })
    } else {
        Ok(())
    }
}
