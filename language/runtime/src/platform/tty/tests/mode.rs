use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, close_tty_worker_resource,
    open_pty_or_skip_not_supported, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, TtyHandle};
use crate::platform::tty::TtyMode;

/// Convert one platform tty flag into `u64` for test bit arithmetic.
#[cfg(unix)]
#[allow(clippy::useless_conversion)]
fn tty_flag_u64(flag: libc::tcflag_t) -> u64 {
    flag.into()
}

/// Roundtrip tty mode on one opened worker endpoint.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_mode_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        // open one pty pair or skip when unsupported
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        // read one mode snapshot and reapply it
        let mode = context.destack_tty_get_mode(pair.worker)?;
        let mode = super::decode_harness_value(mode);
        let mode_value = context.tty_mode_value(mode);
        context.destack_tty_set_mode(pair.worker, mode_value)?;

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject unknown tty handles for mode operations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_mode_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId(0));

        let get_result = context.destack_tty_get_mode(unknown);
        assert_platform_error_codes(get_result, &[PlatformErrorCode::InvalidArgumentValue])?;

        let mode = TtyMode {
            input_flags: 0,
            output_flags: 0,
            control_flags: 0,
            local_flags: 0,
        };
        let mode_value = context.tty_mode_value(mode);
        let set_result = context.destack_tty_set_mode(unknown, mode_value);
        assert_platform_error_codes(set_result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject unsupported non-local mode fields on Windows.
#[cfg(windows)]
#[test]
fn test_tty_mode_rejects_non_local_fields_on_windows() {
    with_harness_context(|mut context| {
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        let unsupported = TtyMode {
            input_flags: 1,
            output_flags: 0,
            control_flags: 0,
            local_flags: 0,
        };
        let unsupported = context.tty_mode_value(unsupported);
        let result = context.destack_tty_set_mode(pair.worker, unsupported);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])?;

        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Toggle raw mode on one opened worker endpoint.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_mode_set_raw_mode_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        let enabled = assert_ok_or_expected_error(
            context.destack_tty_set_raw_mode(pair.worker, true),
            &[PlatformErrorCode::NotSupported],
        )?;
        if enabled.is_none() {
            context.destack_tty_pty_close(pair.controller)?;
            close_tty_worker_resource(context.call_context, pair.worker)?;
            return Ok(());
        }

        let raw_mode = context.destack_tty_get_mode(pair.worker)?;
        let raw_mode = super::decode_harness_value(raw_mode);
        #[cfg(unix)]
        {
            let raw_mask = tty_flag_u64(libc::ICANON | libc::ECHO | libc::ISIG | libc::IEXTEN);
            assert_eq!(raw_mode.local_flags & raw_mask, 0);
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{
                ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
            };

            let raw_mask = (ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT) as u64;
            assert_eq!(raw_mode.local_flags & raw_mask, 0);
        }

        context.destack_tty_set_raw_mode(pair.worker, false)?;
        let cooked_mode = context.destack_tty_get_mode(pair.worker)?;
        let cooked_mode = super::decode_harness_value(cooked_mode);
        #[cfg(unix)]
        {
            let cooked_mask = tty_flag_u64(libc::ICANON | libc::ECHO);
            assert_eq!(cooked_mode.local_flags & cooked_mask, cooked_mask);
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::Console::{
                ENABLE_ECHO_INPUT, ENABLE_LINE_INPUT, ENABLE_PROCESSED_INPUT,
            };

            let cooked_mask =
                (ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT) as u64;
            assert_eq!(cooked_mode.local_flags & cooked_mask, cooked_mask);
        }

        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject unknown tty handles for raw-mode operations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_mode_set_raw_mode_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId(0));
        let result = context.destack_tty_set_raw_mode(unknown, true);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}
