use super::{
    assert_ok_or_expected_error, assert_platform_error_codes, close_tty_worker_resource,
    decode_harness_value, with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{ResourceId, TtyHandle};
use crate::platform::tty::TtySize;

/// Roundtrip tty size on one opened worker endpoint.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_size_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        // open one pty pair or skip when unsupported
        let pair = assert_ok_or_expected_error(
            context.destack_tty_pty_open(24, 80, 0),
            &[PlatformErrorCode::NotSupported],
        )?;
        let Some(pair) = pair else {
            return Ok(());
        };
        let pair = decode_harness_value(pair);

        // read one size snapshot and reapply it
        let size = context.destack_tty_get_size(pair.worker)?;
        let size = decode_harness_value(size);
        if size.rows > 0 && size.columns > 0 {
            let size_value = context.tty_size_value(size);
            context.destack_tty_set_size(pair.worker, size_value)?;
        }

        // close resources
        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject zero dimensions for size updates.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_set_size_rejects_zero_dimensions() {
    with_harness_context(|mut context| {
        let open_result = context.destack_tty_pty_open(24, 80, 0);
        let pair = assert_ok_or_expected_error(open_result, &[PlatformErrorCode::NotSupported])?;
        let Some(pair) = pair else {
            return Ok(());
        };
        let pair = decode_harness_value(pair);

        let invalid_size = TtySize {
            rows: 0,
            columns: 80,
            x_pixels: 0,
            y_pixels: 0,
        };
        let size_value = context.tty_size_value(invalid_size);
        let set_result = context.destack_tty_set_size(pair.worker, size_value);
        assert_platform_error_codes(
            set_result,
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)
    });
}

/// Roundtrip one explicit size update for one opened worker endpoint.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_size_updates_roundtrip_or_not_supported() {
    with_harness_context(|mut context| {
        let pair = assert_ok_or_expected_error(
            context.destack_tty_pty_open(24, 80, 0),
            &[PlatformErrorCode::NotSupported],
        )?;
        let Some(pair) = pair else {
            return Ok(());
        };
        let pair = decode_harness_value(pair);

        let target_size = TtySize {
            rows: 30,
            columns: 100,
            x_pixels: 0,
            y_pixels: 0,
        };
        let target_value = context.tty_size_value(target_size);
        context.destack_tty_set_size(pair.worker, target_value)?;

        let updated = context.destack_tty_get_size(pair.worker)?;
        let updated = decode_harness_value(updated);
        assert_eq!(updated.rows, target_size.rows);
        assert_eq!(updated.columns, target_size.columns);

        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}

/// Reject unknown tty handles for size operations.
#[cfg(any(unix, windows))]
#[test]
fn test_tty_size_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let unknown = TtyHandle(ResourceId(0));

        let get_result = context.destack_tty_get_size(unknown);
        assert_platform_error_codes(
            get_result,
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )?;

        let size = TtySize {
            rows: 24,
            columns: 80,
            x_pixels: 0,
            y_pixels: 0,
        };
        let size_value = context.tty_size_value(size);
        let set_result = context.destack_tty_set_size(unknown, size_value);
        assert_platform_error_codes(
            set_result,
            &[
                PlatformErrorCode::InvalidArgumentValue,
                PlatformErrorCode::NotSupported,
            ],
        )
    });
}

/// Report zero pixel metrics for Windows pseudo-terminal size snapshots.
#[cfg(windows)]
#[test]
fn test_tty_size_windows_pseudo_console_pixels_are_zero() {
    with_harness_context(|mut context| {
        let pair = assert_ok_or_expected_error(
            context.destack_tty_pty_open(24, 80, 0),
            &[PlatformErrorCode::NotSupported],
        )?;
        let Some(pair) = pair else {
            return Ok(());
        };
        let pair = decode_harness_value(pair);

        let requested = TtySize {
            rows: 25,
            columns: 90,
            x_pixels: 640,
            y_pixels: 480,
        };
        let requested = context.tty_size_value(requested);
        context.destack_tty_set_size(pair.worker, requested)?;

        let observed = context.destack_tty_get_size(pair.worker)?;
        let observed = decode_harness_value(observed);
        assert_eq!(observed.x_pixels, 0);
        assert_eq!(observed.y_pixels, 0);

        context.destack_tty_pty_close(pair.controller)?;
        close_tty_worker_resource(context.call_context, pair.worker)?;

        Ok(())
    });
}
