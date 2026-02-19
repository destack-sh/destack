use super::{assert_platform_error_code, with_harness_context};

use crate::platform::diagnostic::PlatformErrorCode;

/// Reject invalid synchronization parameters with explicit argument errors.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_sync_rejects_invalid_parameters() {
    with_harness_context(|mut context| {
        let mutex_result = context.destack_thread_mutex_create(1);
        assert_platform_error_code(mutex_result, PlatformErrorCode::InvalidArgumentValue)?;

        let semaphore_result = context.destack_thread_semaphore_create(0, 0, 0);
        assert_platform_error_code(semaphore_result, PlatformErrorCode::InvalidArgumentValue)?;

        let barrier_result = context.destack_thread_barrier_create(0, 0);
        assert_platform_error_code(barrier_result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Reject one zero address for wait-address primitives.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_address_wait_rejects_zero_address() {
    with_harness_context(|mut context| {
        let wait_result = context.destack_thread_address_wait(0, 0, 0);
        assert_platform_error_code(wait_result, PlatformErrorCode::InvalidArgumentValue)?;

        let wake_one_result = context.destack_thread_address_wake_one(0);
        assert_platform_error_code(wake_one_result, PlatformErrorCode::InvalidArgumentValue)?;

        let wake_all_result = context.destack_thread_address_wake_all(0);
        assert_platform_error_code(wake_all_result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}
