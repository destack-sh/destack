use crate::{HeapConfigurationError, HeapError, SizeClassTable, SizeClassTableError};

/// The largest page or chunk width stored by allocator metadata.
const MAX_ALLOCATOR_WIDTH_BYTES: usize = u32::MAX as usize;

/// Validate one configured allocator page size.
pub(crate) fn validate_page_size_bytes(page_size_bytes: usize) -> Result<usize, HeapError> {
    if page_size_bytes == 0
        || page_size_bytes > MAX_ALLOCATOR_WIDTH_BYTES
        || !page_size_bytes.is_power_of_two()
    {
        Err(HeapError::configuration(
            HeapConfigurationError::InvalidPageSizeBytes {
                bytes: page_size_bytes,
            },
        ))
    } else {
        Ok(page_size_bytes)
    }
}

/// Validate one configured allocator chunk size against one allocator page size.
pub(crate) fn validate_allocator_chunk_size_bytes(
    page_size_bytes: usize,
    allocator_chunk_size_bytes: usize,
) -> Result<usize, HeapError> {
    if allocator_chunk_size_bytes == 0 || allocator_chunk_size_bytes > MAX_ALLOCATOR_WIDTH_BYTES {
        return Err(HeapError::configuration(
            HeapConfigurationError::InvalidAllocatorChunkSizeBytes {
                bytes: allocator_chunk_size_bytes,
            },
        ));
    }

    if !allocator_chunk_size_bytes.is_power_of_two() {
        return Err(HeapError::configuration(
            HeapConfigurationError::InvalidAllocatorChunkSizeBytes {
                bytes: allocator_chunk_size_bytes,
            },
        ));
    }

    if !allocator_chunk_size_bytes.is_multiple_of(page_size_bytes) {
        return Err(HeapError::configuration(
            HeapConfigurationError::MisalignedAllocatorChunkSize {
                page_size_bytes,
                chunk_size_bytes: allocator_chunk_size_bytes,
            },
        ));
    }

    Ok(allocator_chunk_size_bytes)
}

/// Validate one configured virtual address-space size against one allocator page size.
pub(crate) fn validate_address_space_size_bytes(
    page_size_bytes: usize,
    address_space_size_bytes: usize,
) -> Result<usize, HeapError> {
    if address_space_size_bytes == 0 {
        return Err(HeapError::configuration(
            HeapConfigurationError::InvalidAddressSpaceSizeBytes {
                bytes: address_space_size_bytes,
            },
        ));
    }

    if !address_space_size_bytes.is_multiple_of(page_size_bytes) {
        return Err(HeapError::configuration(
            HeapConfigurationError::MisalignedAddressSpaceSize {
                page_size_bytes,
                address_space_size_bytes,
            },
        ));
    }

    Ok(address_space_size_bytes)
}

/// Validate one configured small-block alignment.
pub(crate) fn validate_small_allocation_alignment_bytes(
    alignment_bytes: usize,
) -> Result<usize, HeapError> {
    if alignment_bytes == 0 || !alignment_bytes.is_power_of_two() {
        Err(HeapError::configuration(
            HeapConfigurationError::InvalidSmallAllocationAlignmentBytes {
                bytes: alignment_bytes,
            },
        ))
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

    for class in size_classes.classes.iter() {
        if class.bytes % alignment_bytes != 0 {
            return Err(HeapError::configuration(
                HeapConfigurationError::MisalignedSizeClass {
                    alignment_bytes,
                    class_bytes: class.bytes,
                },
            ));
        }
    }

    Ok(())
}

/// Validate one small span against one size-class table.
pub(crate) fn validate_small_span_size_bytes(
    span_size_bytes: usize,
    size_classes: &SizeClassTable,
) -> Result<(), HeapError> {
    let max_small_bytes =
        size_classes
            .max_small_allocation_bytes()
            .ok_or(HeapError::configuration(
                HeapConfigurationError::InvalidSizeClassTable {
                    reason: SizeClassTableError::Empty,
                },
            ))?;

    if span_size_bytes < max_small_bytes {
        Err(HeapError::configuration(
            HeapConfigurationError::SmallSpanTooSmall {
                span_size_bytes,
                class_bytes: max_small_bytes,
            },
        ))
    } else {
        Ok(())
    }
}
