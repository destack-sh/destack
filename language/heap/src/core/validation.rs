use crate::{HeapConfigurationError, HeapError, SizeClassTable, SizeClassTableError};

/// The largest page width stored by heap metadata.
const MAX_HEAP_PAGE_SIZE_BYTES: usize = u32::MAX as usize;

/// Validate one configured heap page size.
pub(crate) fn validate_page_size_bytes(page_size_bytes: usize) -> Result<usize, HeapError> {
    if page_size_bytes == 0
        || page_size_bytes > MAX_HEAP_PAGE_SIZE_BYTES
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
