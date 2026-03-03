use super::{
    HarnessValue, HarnessWindowMode, decode_harness_value, default_window_options, error_code,
    harness_window_logical_size, harness_window_mode_options, harness_window_physical_size,
    with_harness_context,
};
#[cfg(windows)]
use super::{harness_window_icon_set, harness_window_icon_set_none};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::WindowVisibility;
#[cfg(windows)]
use crate::platform::display::{WindowAspectRatio, WindowAspectRatioVm};
use crate::platform::resource::{DisplayHandle, ResourceId};

#[cfg(windows)]
use windows_sys::Win32::Foundation::POINT;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSORINFO, GWL_STYLE, GetCursorInfo, GetWindowLongPtrW, WS_DISABLED,
};

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
fn test_window_mode_exclusive_with_invalid_display_is_rejected() {
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

        let exclusive = harness_window_mode_options(
            &context,
            HarnessWindowMode::ExclusiveFullscreen {
                display: DisplayHandle(ResourceId(0)),
                display_mode: None,
            },
        );
        let result = context.destack_display_window_set_mode(window, exclusive);
        let error = result.expect_err("exclusive mode with invalid target display should fail");
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::IoNotFound)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[test]
fn test_window_open_mode_exclusive_with_invalid_display_is_rejected() {
    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "exclusive-open")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.mode = crate::platform::display::WindowModeOptions::WindowExclusiveFullscreenModeOptions(
                    crate::platform::display::WindowExclusiveFullscreenModeOptions {
                        kind: context.call_context.store_string("exclusiveFullscreen"),
                        display: DisplayHandle(ResourceId(0)),
                        display_mode: None,
                    },
                );
            }
            HarnessValue::Vm(options) => {
                let vm_context = context
                    .vm_context
                    .map(|vm_context| unsafe {
                        &mut *(vm_context as *mut destack_vm::ExternalCallContext<'_>)
                    })
                    .expect("vm context should exist for vm harness");
                options.mode = crate::platform::display::WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(
                    crate::platform::display::WindowExclusiveFullscreenModeOptionsVm {
                        kind: destack_vm::StringHandle::new(
                            vm_context.intern_string("exclusiveFullscreen"),
                        ),
                        display: DisplayHandle(ResourceId(0)),
                        display_mode: None,
                    },
                );
            }
        }

        let result = context.destack_display_window_open(options);
        let error = result.expect_err("exclusive open with invalid target display should fail");
        if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
            return Ok(());
        }
        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::IoNotFound)
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
fn test_window_open_size_matches_requested_client_size() {
    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "open-client-size")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.size_logical.width = 900.0;
                options.size_logical.height = 540.0;
            }
            HarnessValue::Vm(options) => {
                options.size_logical.width = 900.0;
                options.size_logical.height = 540.0;
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
        assert!((state.size_logical.width - 900.0).abs() <= 1.0);
        assert!((state.size_logical.height - 540.0).abs() <= 1.0);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_mode_borderless_without_display_is_accepted() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "borderless-no-display")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let borderless = harness_window_mode_options(&context, HarnessWindowMode::Borderless);
        context.destack_display_window_set_mode(window, borderless)?;

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
fn test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode() {
    with_harness_context(|mut context| {
        let first_options = default_window_options(&mut context, "cursor-hidden-first")?;
        let first_window = match context.destack_display_window_open(first_options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let second_options = default_window_options(&mut context, "cursor-hidden-second")?;
        let second_window = context.destack_display_window_open(second_options)?;

        context.destack_display_window_set_cursor_mode(
            first_window,
            crate::platform::display::WindowCursorMode::Hidden,
        )?;
        context.destack_display_window_set_cursor_mode(
            second_window,
            crate::platform::display::WindowCursorMode::Hidden,
        )?;

        context.destack_display_window_close(first_window)?;

        let mut cursor = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            flags: 0,
            hCursor: 0,
            ptScreenPos: POINT { x: 0, y: 0 },
        };
        let status = unsafe { GetCursorInfo(&mut cursor) };
        assert_ne!(status, 0);
        assert_eq!(cursor.flags & CURSOR_SHOWING, 0);

        context.destack_display_window_close(second_window)?;

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

#[cfg(windows)]
#[test]
fn test_window_aspect_ratio_roundtrip_and_size_lock() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "aspect-ratio")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let aspect_ratio = if context.vm_context.is_some() {
            HarnessValue::Vm(Some(WindowAspectRatioVm {
                numerator: 16,
                denominator: 9,
            }))
        } else {
            HarnessValue::Native(Some(WindowAspectRatio {
                numerator: 16,
                denominator: 9,
            }))
        };
        context.destack_display_window_set_aspect_ratio(window, aspect_ratio)?;

        let logical_size = harness_window_logical_size(&context, 1280.0, 1000.0);
        context.destack_display_window_set_size_logical(window, logical_size)?;

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        let ratio = state.size_logical.width / state.size_logical.height;
        let expected = 16.0 / 9.0;
        assert!((ratio - expected).abs() <= 0.05);
        assert_eq!(
            state.aspect_ratio,
            Some(WindowAspectRatio {
                numerator: 16,
                denominator: 9
            })
        );

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

#[cfg(windows)]
#[test]
fn test_window_icons_set_and_clear() {
    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "icon-set")?;
        let window = match context.destack_display_window_open(options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let icons = harness_window_icon_set(&mut context)?;
        context.destack_display_window_set_icons(window, icons)?;
        context.destack_display_window_set_icons(window, harness_window_icon_set_none(&context))?;

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[test]
fn test_window_modal_parent_transition_reenables_previous_owner() {
    with_harness_context(|mut context| {
        let owner_a_options = default_window_options(&mut context, "owner-a")?;
        let owner_a = match context.destack_display_window_open(owner_a_options) {
            Ok(window) => window,
            Err(error) => {
                if error_code(&error) == Some(PlatformErrorCode::NotSupported) {
                    return Ok(());
                }

                return Err(error);
            }
        };

        let owner_b_options = default_window_options(&mut context, "owner-b")?;
        let owner_b = context.destack_display_window_open(owner_b_options)?;
        let child_options = default_window_options(&mut context, "child-modal")?;
        let child = context.destack_display_window_open(child_options)?;

        context.destack_display_window_set_parent(child, Some(owner_a))?;
        context.destack_display_window_set_modal(child, true)?;

        let owner_a_hwnd = context
            .call_context
            .runtime()
            .resources
            .with_entry(owner_a.0, |entry| entry.raw_handle)
            .flatten()
            .expect("owner-a window resource should expose raw hwnd");
        let owner_b_hwnd = context
            .call_context
            .runtime()
            .resources
            .with_entry(owner_b.0, |entry| entry.raw_handle)
            .flatten()
            .expect("owner-b window resource should expose raw hwnd");

        let owner_a_style = unsafe { GetWindowLongPtrW(owner_a_hwnd as isize, GWL_STYLE) } as u32;
        assert_ne!(owner_a_style & WS_DISABLED, 0);

        context.destack_display_window_set_parent(child, Some(owner_b))?;
        let owner_a_style = unsafe { GetWindowLongPtrW(owner_a_hwnd as isize, GWL_STYLE) } as u32;
        let owner_b_style = unsafe { GetWindowLongPtrW(owner_b_hwnd as isize, GWL_STYLE) } as u32;
        assert_eq!(owner_a_style & WS_DISABLED, 0);
        assert_ne!(owner_b_style & WS_DISABLED, 0);

        context.destack_display_window_set_modal(child, false)?;
        let owner_b_style = unsafe { GetWindowLongPtrW(owner_b_hwnd as isize, GWL_STYLE) } as u32;
        assert_eq!(owner_b_style & WS_DISABLED, 0);

        context.destack_display_window_close(child)?;
        context.destack_display_window_close(owner_b)?;
        context.destack_display_window_close(owner_a)?;
        Ok(())
    });
}
