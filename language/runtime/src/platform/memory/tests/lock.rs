use super::{
    assert_ok_or_expected_error, decode_memory_range, page_aligned_length, protection_read_write,
    reserve_flags_none, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;

/// Lock and unlock one mapping when the host allows it.
#[cfg(any(unix, windows))]
#[test]
fn test_memory_lock_unlock_or_expected_error() {
    with_harness_context(|mut context| {
        let page_size = context.destack_memory_page_size()?;
        let length = page_aligned_length(page_size);

        let mapping = context.destack_memory_reserve(length, 0, reserve_flags_none())?;
        let mapping = decode_memory_range(mapping);
        context.destack_memory_commit(mapping.address, mapping.length, protection_read_write())?;

        let locked = assert_ok_or_expected_error(
            context.destack_memory_lock(mapping.address, mapping.length),
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        if locked.is_some() {
            let _ = assert_ok_or_expected_error(
                context.destack_memory_unlock(mapping.address, mapping.length),
                &[PlatformErrorCode::NotSupported],
            )?;
        }

        context.destack_memory_release(mapping.address, mapping.length)?;

        Ok(())
    });
}
