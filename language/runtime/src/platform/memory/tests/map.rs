use super::{
    assert_platform_error_codes, decode_memory_range, page_aligned_length, protection_read_write,
    reserve_flags_none, with_harness_context, writable_byte_pointer,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Reserve, commit, decommit, and release one mapping.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_map_roundtrip_reserve_commit_release() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        assert_ne!(mapping.address, 0);
        assert_eq!(mapping.length, length);
        assert_eq!(mapping.address % page_size, 0);

        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        // touch one byte in committed memory
        unsafe {
            let pointer = writable_byte_pointer(mapping.address);
            pointer.write_volatile(0x2A);
            assert_eq!(pointer.read_volatile(), 0x2A);
        }

        context.destack_memory_decommit(mapping.address, mapping.length)?;
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        // verify writable memory after recommit
        unsafe {
            let pointer = writable_byte_pointer(mapping.address);
            pointer.write_volatile(0x5A);
            assert_eq!(pointer.read_volatile(), 0x5A);
        }

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}

/// Reject zero-length reserve requests.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_map_rejects_zero_length_reserve() {
    with_harness_context(|mut context| {
        let result = context.destack_memory_reserve(0, 0, reserve_flags_none());
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject misaligned commit requests.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_map_rejects_misaligned_commit() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);
        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);

        let result = context.destack_memory_commit(
            mapping.address + 1,
            mapping.length,
            protection_read_write(),
        );
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])?;

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}
