use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource;

use super::core::{assert_platform_error_code, unique_ipc_name};
use super::with_harness_context;

/// Verify semaphore create, post, and wait behavior across native and VM bindings.
#[cfg(any(unix, windows))]
#[test]
fn test_semaphore_create_post_wait() {
    with_harness_context(|mut context| {
        // create one named semaphore with zero initial permits
        let name = unique_ipc_name("ipc_semaphore_roundtrip");
        let name = context.string_value(&name)?;
        let handle = context.destack_ipc_semaphore_create(name, 0, 0)?;

        // immediate wait should time out with zero permits
        let timeout_error = context
            .destack_ipc_semaphore_wait(handle, 0)
            .err()
            .expect("expected immediate semaphore wait to time out");
        assert_platform_error_code(&timeout_error, PlatformErrorCode::IoTimedOut);

        // post one permit and verify wait now succeeds
        context.destack_ipc_semaphore_post(handle, 1)?;
        context.destack_ipc_semaphore_wait(handle, 1_000_000_000)?;

        Ok(())
    });
}

/// Verify semaphore create rejects unsupported non-zero flags.
#[cfg(any(unix, windows))]
#[test]
fn test_semaphore_create_rejects_unsupported_flags() {
    with_harness_context(|mut context| {
        let name = unique_ipc_name("ipc_semaphore_flags");
        let name = context.string_value(&name)?;
        let error = context
            .destack_ipc_semaphore_create(name, 0, 1)
            .err()
            .expect("expected semaphoreCreate to fail for unsupported flags");
        assert_platform_error_code(&error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify semaphore post and wait reject unknown handles.
#[cfg(any(unix, windows))]
#[test]
fn test_semaphore_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        // build one unknown semaphore handle
        let unknown = resource::SemaphoreHandle(resource::ResourceId(0));

        // post should fail with invalid-argument for unknown handle
        let post_error = context
            .destack_ipc_semaphore_post(unknown, 1)
            .err()
            .expect("expected semaphorePost to fail for unknown handle");
        assert_platform_error_code(&post_error, PlatformErrorCode::InvalidArgumentValue);

        // wait should fail with invalid-argument for unknown handle
        let wait_error = context
            .destack_ipc_semaphore_wait(unknown, 0)
            .err()
            .expect("expected semaphoreWait to fail for unknown handle");
        assert_platform_error_code(&wait_error, PlatformErrorCode::InvalidArgumentValue);

        Ok(())
    });
}

/// Verify futex wait and wake behavior on linux-style futex hosts.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn test_futex_wait_and_wake_paths() {
    with_harness_context(|mut context| {
        // create and map one shared-memory object for futex operations
        let name = unique_ipc_name("ipc_futex_wait_wake");
        let name = context.string_value(&name)?;
        let handle = context.destack_ipc_shared_memory_create(name, 4096, 0)?;
        let mapping = super::core::decode_mapping_value(
            context.destack_ipc_shared_memory_map(handle, 0, 4096, 0)?,
        );

        // initialize one futex word and verify mismatch waits would-block
        unsafe {
            let word = mapping.address as usize as *mut u32;
            *word = 1;
        }
        let mismatch_error = context
            .destack_ipc_futex_wait(handle, 0, 0, 0)
            .err()
            .expect("expected futexWait to report wouldBlock on mismatch");
        assert_platform_error_code(&mismatch_error, PlatformErrorCode::IoWouldBlock);

        // verify wake reports zero when no waiter is currently blocked
        let woken = context.destack_ipc_futex_wake(handle, 0, 1)?;
        assert_eq!(woken, 0);

        // verify timed wait reports ioTimedOut when no wake occurs
        let timeout_error = context
            .destack_ipc_futex_wait(handle, 0, 1, 2_000_000)
            .err()
            .expect("expected futexWait to time out without a wake");
        assert_platform_error_code(&timeout_error, PlatformErrorCode::IoTimedOut);

        // unmap and close one futex shared-memory object
        context.destack_ipc_shared_memory_unmap(mapping.address, mapping.length)?;
        context.destack_ipc_shared_memory_close(handle)?;

        Ok(())
    });
}
