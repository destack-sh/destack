use super::{
    HarnessValue, HarnessWindowMode, decode_harness_value, default_window_options, error_code,
    harness_window_icon_set, harness_window_icon_set_none, harness_window_logical_size,
    harness_window_mode_options, harness_window_physical_size, is_not_supported_code,
    open_window_or_skip_not_supported, run_execution_case_or_return, wait_window_visibility,
    with_harness_context,
};
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{display as display_platform, resource};
#[cfg(windows)]
use display_platform::{WindowAspectRatio, WindowAspectRatioVm};
use display_platform::{
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowRole, WindowVisibility,
};
use resource::{DisplayHandle, ResourceId};

#[cfg(windows)]
use std::thread::sleep;
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use windows_sys::Win32::Foundation::POINT;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CURSOR_SHOWING, CURSORINFO, GWL_STYLE, GetCursorInfo, GetWindowLongPtrW, WS_DISABLED,
};

#[cfg(windows)]
const CURSOR_VISIBILITY_RETRY_COUNT: usize = 50;
#[cfg(windows)]
const CURSOR_VISIBILITY_RETRY_DELAY_MS: u64 = 10;

#[cfg(windows)]
/// Return whether the process cursor is currently visible.
fn is_cursor_visible() -> bool {
    let mut cursor = CURSORINFO {
        cbSize: std::mem::size_of::<CURSORINFO>() as u32,
        flags: 0,
        hCursor: 0,
        ptScreenPos: POINT { x: 0, y: 0 },
    };
    let status = unsafe { GetCursorInfo(&mut cursor) };
    assert_ne!(status, 0);
    cursor.flags & CURSOR_SHOWING != 0
}

#[cfg(windows)]
/// Wait until the process cursor matches one expected visibility state.
fn wait_cursor_visibility(expected_visible: bool) -> bool {
    // poll cursor visibility with one bounded retry budget
    for _ in 0..CURSOR_VISIBILITY_RETRY_COUNT {
        if is_cursor_visible() == expected_visible {
            return true;
        }

        sleep(Duration::from_millis(CURSOR_VISIBILITY_RETRY_DELAY_MS));
    }

    false
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_rejects_invalid_size_values() {
    if run_execution_case_or_return(display_case_name!(test_window_rejects_invalid_size_values)) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "size-invalid")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_mode_exclusive_with_invalid_display_is_rejected() {
    if run_execution_case_or_return(display_case_name!(
        test_window_mode_exclusive_with_invalid_display_is_rejected
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "exclusive-mode")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_failed_mode_change_preserves_previous_mode() {
    if run_execution_case_or_return(display_case_name!(
        test_window_failed_mode_change_preserves_previous_mode
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "mode-rollback")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let initial_descriptor = context.destack_display_window_descriptor(window)?;
        let initial_is_windowed = match initial_descriptor {
            HarnessValue::Native(value) => matches!(
                value.mode,
                display_platform::WindowModeOptions::WindowWindowedModeOptions(_)
            ),
            HarnessValue::Vm(value) => matches!(
                value.mode,
                display_platform::WindowModeOptionsVm::WindowWindowedModeOptions(_)
            ),
        };
        assert!(initial_is_windowed);

        let invalid_mode = harness_window_mode_options(
            &context,
            HarnessWindowMode::ExclusiveFullscreen {
                display: DisplayHandle(ResourceId(0)),
                display_mode: None,
            },
        );
        let result = context.destack_display_window_set_mode(window, invalid_mode);
        let error = result.expect_err("mode change with invalid display should fail");
        if is_not_supported_code(error_code(&error)) {
            let next_descriptor = context.destack_display_window_descriptor(window)?;
            let next_is_windowed = match next_descriptor {
                HarnessValue::Native(value) => matches!(
                    value.mode,
                    display_platform::WindowModeOptions::WindowWindowedModeOptions(_)
                ),
                HarnessValue::Vm(value) => matches!(
                    value.mode,
                    display_platform::WindowModeOptionsVm::WindowWindowedModeOptions(_)
                ),
            };
            assert!(next_is_windowed);
            context.destack_display_window_close(window)?;
            return Ok(());
        }

        assert!(matches!(
            error_code(&error),
            Some(PlatformErrorCode::InvalidArgument)
                | Some(PlatformErrorCode::IoNotFound)
                | Some(PlatformErrorCode::InvalidArgumentValue)
        ));

        let next_descriptor = context.destack_display_window_descriptor(window)?;
        let next_is_windowed = match next_descriptor {
            HarnessValue::Native(value) => matches!(
                value.mode,
                display_platform::WindowModeOptions::WindowWindowedModeOptions(_)
            ),
            HarnessValue::Vm(value) => matches!(
                value.mode,
                display_platform::WindowModeOptionsVm::WindowWindowedModeOptions(_)
            ),
        };
        assert!(next_is_windowed);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_open_mode_exclusive_with_invalid_display_is_rejected() {
    if run_execution_case_or_return(display_case_name!(
        test_window_open_mode_exclusive_with_invalid_display_is_rejected
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "exclusive-open")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.mode =
                    display_platform::WindowModeOptions::WindowExclusiveFullscreenModeOptions(
                        display_platform::WindowExclusiveFullscreenModeOptions {
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
                        &mut *(vm_context as *mut destack_vm::BindingContext<'_>)
                    })
                    .expect("vm context should exist for vm harness");
                options.mode =
                    display_platform::WindowModeOptionsVm::WindowExclusiveFullscreenModeOptions(
                        display_platform::WindowExclusiveFullscreenModeOptionsVm {
                            kind: destack_vm::StringHandle::new(
                                vm_context
                                    .intern_string("exclusiveFullscreen")
                                    .expect("vm test string should intern"),
                            ),
                            display: DisplayHandle(ResourceId(0)),
                            display_mode: None,
                        },
                    );
            }
        }

        let result = context.destack_display_window_open(options);
        let error = result.expect_err("exclusive open with invalid target display should fail");
        if is_not_supported_code(error_code(&error)) {
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_open_rejects_unusable_popup_role_configuration() {
    if run_execution_case_or_return(display_case_name!(
        test_window_open_rejects_unusable_popup_role_configuration
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let mut options = default_window_options(&mut context, "role-popup-open")?;
        match &mut options {
            HarnessValue::Native(options) => {
                options.role = WindowRole::Popup;
            }
            HarnessValue::Vm(options) => {
                options.role = WindowRole::Popup;
            }
        }

        let result = context.destack_display_window_open(options);
        let error = result.expect_err("popup role without owner relation should fail");

        if is_not_supported_code(error_code(&error)) {
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_cursor_policy_transitions_keep_close_path_operational() {
    if run_execution_case_or_return(display_case_name!(
        test_window_cursor_policy_transitions_keep_close_path_operational
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "cursor-policy-close")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        // apply icon lane when supported by this backend
        let icon_result =
            context.destack_display_window_set_cursor_icon(window, WindowCursorIcon::Pointer);
        if let Err(error) = icon_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }

        // apply explicit visibility lane when supported by this backend
        let hide_result = context.destack_display_window_set_cursor_visible(window, false);
        if let Err(error) = hide_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }
        context.destack_display_window_set_cursor_visible(window, true)?;

        // apply hidden and normal mode transitions without breaking close behavior
        let mode_result =
            context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Hidden);
        if let Err(error) = mode_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }
        context.destack_display_window_set_cursor_mode(window, WindowCursorMode::Normal)?;

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_visibility_roundtrip_and_double_close_error() {
    if run_execution_case_or_return(display_case_name!(
        test_window_visibility_roundtrip_and_double_close_error
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "visibility")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        // request hidden visibility when supported
        let hidden_result =
            context.destack_display_window_set_visibility(window, WindowVisibility::Hidden);
        if let Err(error) = hidden_result {
            if !is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Err(error);
            }
        } else {
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Hidden,
            )?);
        }

        // minimized visibility should either transition cleanly or report unsupported
        let minimized_result =
            context.destack_display_window_set_visibility(window, WindowVisibility::Minimized);
        if let Err(error) = minimized_result {
            if !is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Err(error);
            }
        } else {
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Minimized,
            )?);

            context.destack_display_window_set_visibility(window, WindowVisibility::Visible)?;
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Visible,
            )?);
        }

        // maximized visibility should be idempotent and restore cleanly back to visible
        let maximized_result =
            context.destack_display_window_set_visibility(window, WindowVisibility::Maximized);
        if let Err(error) = maximized_result {
            if !is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Err(error);
            }
        } else {
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Maximized,
            )?);

            context.destack_display_window_set_visibility(window, WindowVisibility::Maximized)?;
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Maximized,
            )?);

            context.destack_display_window_set_visibility(window, WindowVisibility::Visible)?;
            assert!(wait_window_visibility(
                &mut context,
                window,
                WindowVisibility::Visible,
            )?);
        }

        context.destack_display_window_close(window)?;

        let second_close = context.destack_display_window_close(window);
        let error = second_close.expect_err("second close should fail");
        assert_eq!(error_code(&error), Some(PlatformErrorCode::IoNotFound));

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_set_size_logical_roundtrip_matches_state() {
    if run_execution_case_or_return(display_case_name!(
        test_window_set_size_logical_roundtrip_matches_state
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "size-logical-roundtrip")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let target_size = harness_window_logical_size(&context, 320.0, 180.0);
        context.destack_display_window_set_size_logical(window, target_size)?;

        let state = context.destack_display_window_state(window)?;
        let state = decode_harness_value(state);
        assert_eq!(state.size_logical.width, 320.0);
        assert_eq!(state.size_logical.height, 180.0);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_set_modal_requires_owner_relationship() {
    if run_execution_case_or_return(display_case_name!(
        test_window_set_modal_requires_owner_relationship
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "modal-owner-required")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let result = context.destack_display_window_set_modal(window, true);
        let error = result.expect_err("modal state without owner relationship should fail");
        if is_not_supported_code(error_code(&error)) {
            context.destack_display_window_close(window)?;
            return Ok(());
        }
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_modal_owner_removal_requires_explicit_transition() {
    if run_execution_case_or_return(display_case_name!(
        test_window_modal_owner_removal_requires_explicit_transition
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let owner_options = default_window_options(&mut context, "modal-owner-removal-owner")?;
        let Some(owner) = open_window_or_skip_not_supported(&mut context, owner_options)? else {
            return Ok(());
        };

        let child_options = default_window_options(&mut context, "modal-owner-removal-child")?;
        let Some(child) = open_window_or_skip_not_supported(&mut context, child_options)? else {
            context.destack_display_window_close(owner)?;
            return Ok(());
        };

        let parent_result = context.destack_display_window_set_parent(child, Some(owner));
        if let Err(error) = parent_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(child)?;
                context.destack_display_window_close(owner)?;
                return Ok(());
            }

            context.destack_display_window_close(child)?;
            context.destack_display_window_close(owner)?;
            return Err(error);
        }

        let modal_result = context.destack_display_window_set_modal(child, true);
        if let Err(error) = modal_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_set_parent(child, None)?;
                context.destack_display_window_close(child)?;
                context.destack_display_window_close(owner)?;
                return Ok(());
            }

            context.destack_display_window_set_parent(child, None)?;
            context.destack_display_window_close(child)?;
            context.destack_display_window_close(owner)?;
            return Err(error);
        }

        // verify behavior when removing the final owner while modal is active
        let removal_result = context.destack_display_window_set_parent(child, None);
        match removal_result {
            Ok(()) => {
                let state = decode_harness_value(context.destack_display_window_state(child)?);
                assert_eq!(state.parent, None);
                assert!(!state.modal);
            }
            Err(error) => {
                assert!(matches!(
                    error_code(&error),
                    Some(PlatformErrorCode::InvalidArgument)
                        | Some(PlatformErrorCode::InvalidArgumentValue)
                ));
                context.destack_display_window_set_modal(child, false)?;
                context.destack_display_window_set_parent(child, None)?;
            }
        }

        context.destack_display_window_close(child)?;
        context.destack_display_window_close(owner)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_set_parent_rejects_self_relationship() {
    if run_execution_case_or_return(display_case_name!(
        test_window_set_parent_rejects_self_relationship
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "parent-self-invalid")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let result = context.destack_display_window_set_parent(window, Some(window));
        let error = result.expect_err("window parent relationship should reject self-handle");
        if is_not_supported_code(error_code(&error)) {
            context.destack_display_window_close(window)?;
            return Ok(());
        }
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_parent_and_transient_relationship_roundtrip() {
    if run_execution_case_or_return(display_case_name!(
        test_window_parent_and_transient_relationship_roundtrip
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let owner_options = default_window_options(&mut context, "relation-owner")?;
        let Some(owner) = open_window_or_skip_not_supported(&mut context, owner_options)? else {
            return Ok(());
        };

        let child_options = default_window_options(&mut context, "relation-child")?;
        let Some(child) = open_window_or_skip_not_supported(&mut context, child_options)? else {
            context.destack_display_window_close(owner)?;
            return Ok(());
        };

        let parent_result = context.destack_display_window_set_parent(child, Some(owner));
        if let Err(error) = parent_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(child)?;
                context.destack_display_window_close(owner)?;
                return Ok(());
            }

            context.destack_display_window_close(child)?;
            context.destack_display_window_close(owner)?;
            return Err(error);
        }

        let state = decode_harness_value(context.destack_display_window_state(child)?);
        assert_eq!(state.parent, Some(owner));
        assert_eq!(state.transient_for, None);

        context.destack_display_window_set_parent(child, None)?;
        let state = decode_harness_value(context.destack_display_window_state(child)?);
        assert_eq!(state.parent, None);

        let transient_result = context.destack_display_window_set_transient_for(child, Some(owner));
        if let Err(error) = transient_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(child)?;
                context.destack_display_window_close(owner)?;
                return Ok(());
            }

            context.destack_display_window_close(child)?;
            context.destack_display_window_close(owner)?;
            return Err(error);
        }

        let state = decode_harness_value(context.destack_display_window_state(child)?);
        assert_eq!(state.transient_for, Some(owner));
        assert_eq!(state.parent, None);

        context.destack_display_window_set_transient_for(child, None)?;
        let state = decode_harness_value(context.destack_display_window_state(child)?);
        assert_eq!(state.transient_for, None);

        context.destack_display_window_close(child)?;
        context.destack_display_window_close(owner)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_opacity_roundtrip() {
    if run_execution_case_or_return(display_case_name!(test_window_opacity_roundtrip)) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "opacity-roundtrip")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let first_result = context.destack_display_window_set_opacity(window, 0.67);
        if let Err(error) = first_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert!((state.opacity - 0.67).abs() <= 0.05);

        context.destack_display_window_set_opacity(window, 1.0)?;
        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert!((state.opacity - 1.0).abs() <= 0.05);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_chrome_and_decoration_roundtrip() {
    if run_execution_case_or_return(display_case_name!(
        test_window_chrome_and_decoration_roundtrip
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "chrome-roundtrip")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let chrome_result =
            context.destack_display_window_set_chrome(window, WindowChromeKind::Popup);
        if let Err(error) = chrome_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert_eq!(state.chrome, WindowChromeKind::Popup);

        let undecorated_result = context.destack_display_window_set_decorated(window, false);
        if let Err(error) = undecorated_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let descriptor = context.destack_display_window_descriptor(window)?;
        let decorated = match descriptor {
            HarnessValue::Native(value) => value.decorated,
            HarnessValue::Vm(value) => value.decorated,
        };
        assert!(!decorated);

        context.destack_display_window_set_decorated(window, true)?;
        let descriptor = context.destack_display_window_descriptor(window)?;
        let decorated = match descriptor {
            HarnessValue::Native(value) => value.decorated,
            HarnessValue::Vm(value) => value.decorated,
        };
        assert!(decorated);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(any(windows, target_os = "macos"))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_set_size_physical_matches_client_size() {
    if run_execution_case_or_return(display_case_name!(
        test_window_set_size_physical_matches_client_size
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "client-size")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_open_size_matches_requested_client_size() {
    if run_execution_case_or_return(display_case_name!(
        test_window_open_size_matches_requested_client_size
    )) {
        return;
    }

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

        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert!((state.size_logical.width - 900.0).abs() <= 1.0);
        assert!((state.size_logical.height - 540.0).abs() <= 1.0);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_mode_borderless_without_display_is_accepted() {
    if run_execution_case_or_return(display_case_name!(
        test_window_mode_borderless_without_display_is_accepted
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "borderless-no-display")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let borderless = harness_window_mode_options(&context, HarnessWindowMode::Borderless);
        context.destack_display_window_set_mode(window, borderless)?;

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_focus_on_show_false_does_not_force_focus() {
    if run_execution_case_or_return(display_case_name!(
        test_window_focus_on_show_false_does_not_force_focus
    )) {
        return;
    }

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

        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let state = decode_harness_value(context.destack_display_window_state(window)?);
        assert!(!state.focused);

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode() {
    if run_execution_case_or_return(display_case_name!(
        test_window_close_keeps_cursor_hidden_when_another_window_requests_hidden_mode
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let first_options = default_window_options(&mut context, "cursor-hidden-first")?;
        let Some(first_window) = open_window_or_skip_not_supported(&mut context, first_options)?
        else {
            return Ok(());
        };

        let second_options = default_window_options(&mut context, "cursor-hidden-second")?;
        let second_window = context.destack_display_window_open(second_options)?;

        context.destack_display_window_set_cursor_mode(
            first_window,
            display_platform::WindowCursorMode::Hidden,
        )?;
        context.destack_display_window_set_cursor_mode(
            second_window,
            display_platform::WindowCursorMode::Hidden,
        )?;

        context.destack_display_window_close(first_window)?;
        assert!(
            wait_cursor_visibility(false),
            "cursor should remain hidden while one hidden-mode window remains open"
        );

        context.destack_display_window_close(second_window)?;
        assert!(
            wait_cursor_visibility(true),
            "cursor should be restored after the final hidden-mode window closes"
        );

        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_aspect_ratio_roundtrip_and_size_lock() {
    if run_execution_case_or_return(display_case_name!(
        test_window_aspect_ratio_roundtrip_and_size_lock
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "aspect-ratio")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
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
#[cfg_attr(test, test)]
pub(crate) fn test_window_close_restores_cursor_visibility() {
    if run_execution_case_or_return(display_case_name!(
        test_window_close_restores_cursor_visibility
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "cursor-restore")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        context.destack_display_window_set_cursor_mode(
            window,
            display_platform::WindowCursorMode::Hidden,
        )?;
        context.destack_display_window_close(window)?;
        assert!(
            wait_cursor_visibility(true),
            "cursor should be visible after closing one hidden-mode window"
        );

        Ok(())
    });
}

#[cfg(any(unix, windows))]
#[cfg_attr(test, test)]
pub(crate) fn test_window_icons_set_and_clear() {
    if run_execution_case_or_return(display_case_name!(test_window_icons_set_and_clear)) {
        return;
    }

    with_harness_context(|mut context| {
        let options = default_window_options(&mut context, "icon-set")?;
        let Some(window) = open_window_or_skip_not_supported(&mut context, options)? else {
            return Ok(());
        };

        let icons = harness_window_icon_set(&mut context)?;
        let set_result = context.destack_display_window_set_icons(window, icons);
        if let Err(error) = set_result {
            if is_not_supported_code(error_code(&error)) {
                context.destack_display_window_close(window)?;
                return Ok(());
            }

            context.destack_display_window_close(window)?;
            return Err(error);
        }

        let clear_result = context
            .destack_display_window_set_icons(window, harness_window_icon_set_none(&context));
        if let Err(error) = clear_result {
            context.destack_display_window_close(window)?;
            return Err(error);
        }

        context.destack_display_window_close(window)?;
        Ok(())
    });
}

#[cfg(windows)]
#[cfg_attr(test, test)]
pub(crate) fn test_window_modal_parent_transition_reenables_previous_owner() {
    if run_execution_case_or_return(display_case_name!(
        test_window_modal_parent_transition_reenables_previous_owner
    )) {
        return;
    }

    with_harness_context(|mut context| {
        let owner_a_options = default_window_options(&mut context, "owner-a")?;
        let Some(owner_a) = open_window_or_skip_not_supported(&mut context, owner_a_options)?
        else {
            return Ok(());
        };

        let owner_b_options = default_window_options(&mut context, "owner-b")?;
        let owner_b = context.destack_display_window_open(owner_b_options)?;
        let child_options = default_window_options(&mut context, "child-modal")?;
        let child = context.destack_display_window_open(child_options)?;

        context.destack_display_window_set_parent(child, Some(owner_a))?;
        context.destack_display_window_set_modal(child, true)?;

        let owner_a_hwnd = context
            .call_context
            .worker()
            .resources
            .with_entry(owner_a.0, |entry| entry.raw_handle)
            .flatten()
            .expect("owner-a window resource should expose raw hwnd");
        let owner_b_hwnd = context
            .call_context
            .worker()
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
