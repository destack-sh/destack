use super::{
    assert_platform_error_code, default_thread_options, test_thread_entry, with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;

/// Spawn one thread and join it to observe the exit-code payload.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_spawn_join_roundtrip() {
    with_harness_context(|mut context| {
        let options = default_thread_options();
        let entry = context.string_value(test_thread_entry());
        let options = context.thread_options_value(options);
        let handle = context.destack_thread_spawn(entry, 41, options)?;

        let exit = context.destack_thread_join(handle)?;

        // join should return the low 32-bit argument payload
        assert_eq!(exit, 41);

        Ok(())
    });
}

/// Detach one spawned thread and reject subsequent joins on the consumed handle.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_detach_consumes_handle() {
    with_harness_context(|mut context| {
        let options = default_thread_options();
        let entry = context.string_value(test_thread_entry());
        let options = context.thread_options_value(options);
        let handle = context.destack_thread_spawn(entry, 0, options)?;

        context.destack_thread_detach(handle)?;

        let result = context.destack_thread_join(handle);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Reject unsupported thread spawn flags.
#[cfg(any(unix, windows))]
#[test]
fn test_thread_spawn_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        let mut options = default_thread_options();
        options.flags = 1;

        let entry = context.string_value(test_thread_entry());
        let options = context.thread_options_value(options);
        let result = context.destack_thread_spawn(entry, 0, options);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}
