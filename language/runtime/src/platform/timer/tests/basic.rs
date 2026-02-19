use super::{
    assert_platform_error_code, assert_platform_error_codes, default_timer_options,
    timer_fd_spec_from_value, with_harness_context,
};

use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::timer::{
    TimerFdClock, TimerFdFlags, TimerFdSetFlags, TimerFdSpec, TimerFlags,
};

/// Schedule and cancel one one-shot timer handle.
#[cfg(any(unix, windows))]
#[test]
fn test_timer_once_cancel_roundtrip() {
    with_harness_context(|mut context| {
        let options = context.timer_options_value(default_timer_options());
        let handle = context.destack_timer_once(1_000_000_000, options)?;

        let is_active = context.destack_timer_is_active(handle)?;
        assert!(is_active);

        let remaining = context.destack_timer_remaining_ns(handle)?;
        assert!(remaining <= 1_000_000_000);

        context.destack_timer_cancel(handle)?;

        let result = context.destack_timer_is_active(handle);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Pause, resume, reset, and update one repeating timer handle.
#[cfg(any(unix, windows))]
#[test]
fn test_timer_control_flow_for_interval_timer() {
    with_harness_context(|mut context| {
        let options = context.timer_options_value(default_timer_options());
        let handle = context.destack_timer_interval(1_000_000_000, options)?;

        context.destack_timer_pause(handle)?;
        let paused_remaining = context.destack_timer_remaining_ns(handle)?;
        assert!(paused_remaining <= 1_000_000_000);

        context.destack_timer_resume(handle)?;
        let resumed_remaining = context.destack_timer_remaining_ns(handle)?;
        assert!(resumed_remaining <= 1_000_000_000);

        context.destack_timer_reset(handle, 500_000_000)?;
        let reset_remaining = context.destack_timer_remaining_ns(handle)?;
        assert!(reset_remaining <= 500_000_000);

        context.destack_timer_update_interval(handle, 250_000_000)?;
        context.destack_timer_cancel(handle)?;

        Ok(())
    });
}

/// Reject zero interval updates.
#[cfg(any(unix, windows))]
#[test]
fn test_timer_update_interval_rejects_zero() {
    with_harness_context(|mut context| {
        let options = context.timer_options_value(default_timer_options());
        let handle = context.destack_timer_interval(1_000_000_000, options)?;

        let result = context.destack_timer_update_interval(handle, 0);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        context.destack_timer_cancel(handle)?;
        Ok(())
    });
}

/// Reject unsupported timer option flags.
#[cfg(any(unix, windows))]
#[test]
fn test_timer_schedule_rejects_unknown_flags() {
    with_harness_context(|mut context| {
        let mut options = default_timer_options();
        options.flags = TimerFlags(1);
        let options = context.timer_options_value(options);

        let result = context.destack_timer_once(1, options);
        assert_platform_error_code(result, PlatformErrorCode::InvalidArgumentValue)?;

        Ok(())
    });
}

/// Exercise timerfd open, set, get, read, and close on supported hosts.
#[cfg(any(unix, windows))]
#[test]
fn test_timer_fd_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        let handle =
            match context.destack_timer_timer_fd_open(TimerFdClock::Monotonic, TimerFdFlags(0)) {
                Ok(handle) => handle,
                Err(error) => {
                    assert_platform_error_code::<crate::platform::resource::TimerFdHandle>(
                        Err(error),
                        PlatformErrorCode::NotSupported,
                    )?;
                    return Ok(());
                }
            };

        let spec = TimerFdSpec {
            initial_ns: 1_000_000,
            interval_ns: 0,
        };
        let spec = context.timer_fd_spec_value(spec);
        context.destack_timer_timer_fd_set(handle, spec, TimerFdSetFlags(0))?;

        let read_result = context.destack_timer_timer_fd_read(handle);
        match read_result {
            Ok(expirations) => assert!(expirations >= 1),
            Err(error) => {
                assert_platform_error_codes::<u64>(
                    Err(error),
                    &[
                        PlatformErrorCode::IoWouldBlock,
                        PlatformErrorCode::NotSupported,
                    ],
                )?;
            }
        }

        let spec = context.destack_timer_timer_fd_get(handle)?;
        let spec = timer_fd_spec_from_value(spec);
        assert_eq!(spec.interval_ns, 0);

        context.destack_timer_timer_fd_close(handle)?;
        Ok(())
    });
}
