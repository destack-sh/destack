use super::{decode_memory_range, page_aligned_length, reserve_flags_none, with_harness_context};

/// Query memory page and allocation metadata.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_query_values() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let granularity = context.destack_memory_allocation_granularity()?;
        let huge_page_size = context.destack_memory_huge_page_size()?;

        assert!(page_size > 0);
        assert!(granularity >= page_size);
        if let Some(huge_page_size) = huge_page_size {
            assert!(huge_page_size >= page_size);
        }

        Ok(())
    });
}

/// Reserve ranges aligned to the reported allocation contract.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_query_alignment_contract() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let granularity = context.destack_memory_allocation_granularity()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        assert_eq!(mapping.address % page_size, 0);
        assert_eq!(mapping.address % granularity, 0);

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}
