use super::{
    decode_harness_value, decode_memory_range, page_aligned_length, protection_read_write,
    remap_flags_may_move, reserve_flags_none, result_or_skip_not_supported, with_harness_context,
};

/// Change page protections and flush instruction cache.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_protect_and_flush_roundtrip() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        context.destack_memory_protect(mapping.address, mapping.length, protection_read_write())?;
        context.destack_memory_flush_instruction_cache(mapping.address, mapping.length)?;

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}

/// Remap one committed range with move permission.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_remap_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        let remapped = result_or_skip_not_supported(context.destack_memory_remap(
            mapping.address,
            mapping.length,
            mapping.length * 2,
            remap_flags_may_move(),
        ))?;

        if let Some(remapped) = remapped {
            let remapped = decode_harness_value(remapped);
            assert_ne!(remapped.address, 0);
            assert_eq!(remapped.length, mapping.length * 2);
            context.destack_memory_release(remapped.address, remapped.length)?;
        } else {
            context.destack_memory_release(mapping.address, mapping.length)?;
        }

        Ok(())
    });
}
