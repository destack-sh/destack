use super::{
    assert_platform_error_codes, close_tty_worker_resource, open_pty_or_skip_not_supported,
    with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{PtyHandle, ResourceId};

/// Open and close one pty pair when host support is available.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_pty_open_close_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        // open one pair or skip when host backend is unavailable
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        // close the controller through the binding and drop worker directly
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject unsupported pty open flags.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_pty_open_rejects_unsupported_flags() {
    with_harness_context(|mut context| {
        let result = context.destack_tty_pty_open(24, 80, 1);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject zero terminal dimensions for pty open.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_pty_open_rejects_zero_dimensions() {
    with_harness_context(|mut context| {
        let zero_rows = context.destack_tty_pty_open(0, 80, 0);
        assert_platform_error_codes(zero_rows, &[PlatformErrorCode::InvalidArgumentValue])?;

        let zero_columns = context.destack_tty_pty_open(24, 0, 0);
        assert_platform_error_codes(zero_columns, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject unknown controller handles for pty close.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_pty_close_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = PtyHandle(ResourceId::local(0));
        let result = context.destack_tty_pty_close(unknown);
        assert_platform_error_codes(result, &[PlatformErrorCode::InvalidArgumentValue])
    });
}

/// Reject closing one pty controller handle twice.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_pty_close_rejects_double_close() {
    with_harness_context(|mut context| {
        let pair = open_pty_or_skip_not_supported(&mut context, 24, 80, 0)?;
        let Some(pair) = pair else {
            return Ok(());
        };

        context.destack_tty_pty_close(pair.controller)?;

        let second_close = context.destack_tty_pty_close(pair.controller);
        assert_platform_error_codes(second_close, &[PlatformErrorCode::InvalidArgumentValue])?;

        close_tty_worker_resource(context.call_context, pair.worker)
    });
}
