use super::{
    HarnessValue, decode_harness_value, default_window_options, error_code,
    harness_window_logical_size, harness_window_mode_options, harness_window_physical_size,
    with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::{WindowMode, WindowVisibility};

#[cfg(windows)]
use windows_sys::Win32::Foundation::POINT;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{CURSOR_SHOWING, CURSORINFO, GetCursorInfo};

#[cfg(any(unix, windows))]
#[test]
fn test_window_rejects_invalid_size_values() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "size-invalid")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let invalid_logical = harness_window_logical_size(&context, 0.0, 100.0);
        let logical_result =
            context.destack_display_window_set_size_logical(window, invalid_logical);
        let logical_error = logical_result.expect_err("zero logical width should fail");
        assert!(matches!(
            error_code(&logical_error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        let invalid_physical = harness_window_physical_size(&context, 0, 100);
        let physical_result =
            context.destack_display_window_set_size_physical(window, invalid_physical);
        let physical_error = physical_result.expect_err("zero physical width should fail");
        assert!(matches!(
            error_code(&physical_error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_mode_exclusive_without_display_is_rejected() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "exclusive-mode")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let exclusive = harness_window_mode_options(&context, WindowMode::ExclusiveFullscreen);
        let result = context.destack_display_window_set_mode(window, exclusive);
        let error = result.expect_err("exclusive mode without target display should fail");
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_open_mode_exclusive_without_display_is_rejected() {
    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "exclusive-open")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.mode.mode = WindowMode::ExclusiveFullscreen;
                options.mode.display = None;
            }
            HarnessValue::Vm(options) => {
                options.mode.mode = WindowMode::ExclusiveFullscreen;
                options.mode.display = None;
            }
        }

        let result = context.destack_display_window_open(options);
        let error = result.expect_err("exclusive open without target display should fail");
        if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
            return Ok(());
        }
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_visibility_roundtrip_and_double_close_error() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "visibility")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        context.destack_display_window_set_visibility(window, WindowVisibility::Hidden)?;
        let hidden_state = context.destack_display_window_state(window)?;
        let hidden_state = decode_harness_value(hidden_state);
        assert_eq!(hidden_state.visibility, WindowVisibility::Hidden);

        context.destack_display_window_set_visibility(window, WindowVisibility::Visible)?;
        let visible_state = context.destack_display_window_state(window)?;
        let visible_state = decode_harness_value(visible_state);
        assert_eq!(visible_state.visibility, WindowVisibility::Visible);

        context.destack_display_window_close(window)?;

        let second_close = context.destack_display_window_close(window);
        let error = second_close.expect_err("second close should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_set_size_physical_matches_client_size() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "client-size")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let requested_size = harness_window_physical_size(&context, 640, 360);
        context.destack_display_window_set_size_physical(window, requested_size)?;
        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.size_physical.width, 640);
        assert_eq!(state.size_physical.height, 360);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_focus_on_show_false_does_not_force_focus() {
    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "focus-disabled")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.focus_on_show = false;
                options.visibility = WindowVisibility::Visible;
            }
            HarnessValue::Vm(options) => {
                options.focus_on_show = false;
                options.visibility = WindowVisibility::Visible;
            }
        }

        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert!(!state.focused);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_close_restores_cursor_visibility() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "cursor-restore")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        context.destack_display_window_set_cursor_mode(
            window,
            crate::platform::display::WindowCursorMode::Hidden,
        )?;
        context.destack_display_window_close(window)?;

        let mut cursor = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            flags: 0,
            hCursor: 0,
            ptScreenPos: POINT { x: 0, y: 0 },
        };
        let status = unsafe { GetCursorInfo(&mut cursor) };
        assert_ne!(status, 0);
        assert_ne!(cursor.flags & CURSOR_SHOWING, 0);

        Ok(())
    });
}
