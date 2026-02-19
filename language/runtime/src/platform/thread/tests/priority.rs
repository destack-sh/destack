use super::{
    assert_platform_error_code, assert_platform_error_codes, default_thread_options,
    test_thread_entry, with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, ThreadHandle};

/// Set and read thread priority and affinity for one spawned thread handle.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_priority_affinity_roundtrip() {
    with_harness_context(|mut context| {
        let options = default_thread_options();
        let entry = context.string_value(test_thread_entry());
        let options = context.thread_options_value(options);
        let handle = context.destack_thread_spawn(entry, 17, options)?;

        let current_priority = context.destack_thread_get_priority(handle)?;
        let set_priority_result = context.destack_thread_set_priority(handle, current_priority);
        if let Err(error) = set_priority_result {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::IoInvalidData,
                ],
            )?;
        } else {
            let priority = context.destack_thread_get_priority(handle)?;
            assert_eq!(priority, current_priority);
        }

        let current_affinity = match context.destack_thread_get_affinity(handle) {
            Ok(affinity) => affinity,
            Err(error) => {
                assert_platform_error_code::<u64>(Err(error), PlatformErrorCode::NotSupported)?;
                let exit = context.destack_thread_join(handle)?;
                assert_eq!(exit, 17);
                return Ok(());
            }
        };
        let target_affinity = if current_affinity == 0 {
            1
        } else {
            current_affinity
        };

        let set_affinity_result = context.destack_thread_set_affinity(handle, target_affinity);
        if let Err(error) = set_affinity_result {
            assert_platform_error_codes::<()>(
                Err(error),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::IoInvalidData,
                ],
            )?;
        } else {
            let affinity = context.destack_thread_get_affinity(handle)?;
            assert_eq!(affinity, target_affinity);
        }

        let exit = context.destack_thread_join(handle)?;

        // join should return the low 32-bit argument payload
        assert_eq!(exit, 17);

        Ok(())
    });
}

/// Reject a zero affinity mask.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_set_affinity_rejects_zero_mask() {
    with_harness_context(|mut context| {
        let options = default_thread_options();
        let entry = context.string_value(test_thread_entry());
        let options = context.thread_options_value(options);
        let handle = context.destack_thread_spawn(entry, 0, options)?;

        let result = context.destack_thread_set_affinity(handle, 0);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        context.destack_thread_detach(handle)?;

        Ok(())
    });
}

/// Reject priority and affinity operations for one unknown thread handle.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_priority_affinity_reject_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = ThreadHandle(ResourceId(0));

        let get_priority_result = context.destack_thread_get_priority(unknown);
        assert_platform_error_code(get_priority_result, PlatformErrorCode::InvalidArgumentValue)?;

        let set_priority_result = context.destack_thread_set_priority(unknown, 0);
        assert_platform_error_code(set_priority_result, PlatformErrorCode::InvalidArgumentValue)?;

        let get_affinity_result = context.destack_thread_get_affinity(unknown);
        assert_platform_error_code(get_affinity_result, PlatformErrorCode::InvalidArgumentValue)?;

        let set_affinity_result = context.destack_thread_set_affinity(unknown, 1);
        assert_platform_error_code(set_affinity_result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}
