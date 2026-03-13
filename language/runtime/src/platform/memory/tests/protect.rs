use super::{
    decode_harness_value, decode_memory_range, page_aligned_length, protection_read_write,
    remap_flags_may_move, reserve_flags_none, result_or_skip_not_supported, with_harness_context,
    writable_byte_pointer,
};
use crate::platform::memory::MEMORY_PROTECTION_READ;

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

/// Preserve written bytes when remapping one read-only range.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_remap_preserves_bytes_from_read_only_source() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        // seed the mapping before switching it to read-only
        unsafe {
            let pointer = writable_byte_pointer(mapping.address);
            pointer.write_volatile(0x6Du8);
        }

        // make the source mapping read-only before remapping
        context.destack_memory_protect(mapping.address, mapping.length, MEMORY_PROTECTION_READ)?;

        let remapped = result_or_skip_not_supported(context.destack_memory_remap(
            mapping.address,
            mapping.length,
            mapping.length * 2,
            remap_flags_may_move(),
        ))?;

        if let Some(remapped) = remapped {
            let remapped = decode_harness_value(remapped);

            // verify the copied prefix survived the remap
            unsafe {
                let pointer = writable_byte_pointer(remapped.address) as *const u8;
                assert_eq!(pointer.read_volatile(), 0x6Du8);
            }

            context.destack_memory_release(remapped.address, remapped.length)?;
        } else {
            context.destack_memory_release(mapping.address, mapping.length)?;
        }

        Ok(())
    });
}
