use super::{
    InputEventRecord, InputHarnessContext, assert_ok_or_expected_error, assert_platform_error_code,
    with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::input::{InputEventAction, InputEventKind, InputReadMode};
use crate::platform::resource::{
    InputDeviceHandle, InputMonitorHandle, ResourceEntry, ResourceId, ResourceKind,
};

/// Open the first listed input device when the host exposes one accessible endpoint.
fn open_first_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // enumerate devices and try each endpoint until one opens
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    for device in devices {
        let opened = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value(&device.id)),
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        if let Some(handle) = opened {
            return Ok(Some(handle));
        }
    }

    Ok(None)
}

/// Open the first listed input device and return metadata when available.
fn open_first_device_with_record_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<(InputDeviceHandle, bool, bool)>> {
    // enumerate devices and try each endpoint until one opens
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    for device in devices {
        let opened = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value(&device.id)),
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        if let Some(handle) = opened {
            return Ok(Some((handle, device.supports_raw, device.supports_text)));
        }
    }

    Ok(None)
}

/// Assert one decoded event payload uses valid runtime fields.
fn assert_event_record_shape(event: &InputEventRecord) {
    assert!(
        !event.device_id.is_empty(),
        "event device id should not be empty"
    );
    assert!(
        event.sequence > 0,
        "event sequence should be monotonic and nonzero"
    );
}

/// Assert one decoded event payload uses coherent kind and action semantics.
fn assert_event_record_semantics(event: &InputEventRecord) {
    assert_event_record_shape(event);

    match event.kind {
        InputEventKind::Key => {
            assert!(
                matches!(
                    event.action,
                    InputEventAction::Press
                        | InputEventAction::Release
                        | InputEventAction::Repeat
                        | InputEventAction::Cancel
                ),
                "key events should use key actions"
            );
        }
        InputEventKind::PointerMotion => {
            assert_eq!(
                event.action,
                InputEventAction::Move,
                "pointer motion should use move action"
            );
        }
        InputEventKind::PointerButton => {
            assert!(
                matches!(
                    event.action,
                    InputEventAction::Press
                        | InputEventAction::Release
                        | InputEventAction::Repeat
                        | InputEventAction::Cancel
                ),
                "pointer button events should use button actions"
            );
        }
        InputEventKind::Scroll => {
            assert_eq!(
                event.action,
                InputEventAction::Scroll,
                "scroll events should use scroll action"
            );
        }
        InputEventKind::Touch => {
            assert!(
                matches!(
                    event.action,
                    InputEventAction::Press
                        | InputEventAction::Release
                        | InputEventAction::Move
                        | InputEventAction::Repeat
                        | InputEventAction::Cancel
                        | InputEventAction::Axis
                ),
                "touch events should use touch-like actions"
            );
        }
        InputEventKind::Gamepad | InputEventKind::Sensor => {}
        InputEventKind::Text => {
            assert_eq!(
                event.action,
                InputEventAction::Text,
                "text events should use text action"
            );
        }
        InputEventKind::Device => {
            assert!(
                matches!(
                    event.action,
                    InputEventAction::Connect
                        | InputEventAction::Disconnect
                        | InputEventAction::Move
                        | InputEventAction::Cancel
                ),
                "device events should use monitor actions"
            );
        }
    }
}

/// Enumerate input devices through the unified harness.
#[test]
fn test_input_list_enumerates_devices() {
    with_harness_context(|mut context| {
        let devices = context.destack_input_list()?;
        let devices = context.device_records_from_value(devices)?;

        for device in devices {
            assert!(!device.id.is_empty(), "device ids should not be empty");
            assert!(!device.name.is_empty(), "device names should not be empty");
        }

        Ok(())
    });
}

/// Reject malformed input device identifiers.
#[test]
fn test_input_open_rejects_invalid_identifier() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_input_open(context.string_value("not-an-input-device")),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject empty input device identifiers.
#[test]
fn test_input_open_rejects_empty_identifier() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_input_open(context.string_value("")),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject input device identifiers that include nul bytes.
#[test]
fn test_input_open_rejects_identifier_with_nul() {
    with_harness_context(|mut context| {
        assert_platform_error_code(
            context.destack_input_open(context.string_value("console\0stdin")),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Open and close explicit windows raw input identifiers.
#[cfg(windows)]
#[test]
fn test_input_open_close_roundtrip_for_windows_raw_ids() {
    with_harness_context(|mut context| {
        for id in ["raw:keyboard", "raw:mouse"] {
            let Some(handle) = assert_ok_or_expected_error(
                context.destack_input_open(context.string_value(id)),
                &[PlatformErrorCode::IoWouldBlock],
            )?
            else {
                continue;
            };
            context.destack_input_close(handle)?;
        }

        Ok(())
    });
}

/// Reject opening duplicate windows raw keyboard handles at the same time.
#[cfg(windows)]
#[test]
fn test_input_open_rejects_second_windows_raw_keyboard_handle() {
    with_harness_context(|mut context| {
        let Some(first) = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value("raw:keyboard")),
            &[PlatformErrorCode::IoWouldBlock],
        )?
        else {
            return Ok(());
        };
        assert_platform_error_code(
            context.destack_input_open(context.string_value("raw:keyboard")),
            PlatformErrorCode::IoWouldBlock,
        )?;
        context.destack_input_close(first)?;

        Ok(())
    });
}

/// Open and close the macos global-session input identifier.
#[cfg(target_os = "macos")]
#[test]
fn test_input_open_close_roundtrip_for_macos_session_id() {
    with_harness_context(|mut context| {
        let handle = context.destack_input_open(context.string_value("macos:session"))?;
        context.destack_input_close(handle)?;

        Ok(())
    });
}

/// Open and close the first available input device when accessible.
#[test]
fn test_input_open_close_roundtrip_for_available_device() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_close(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject close calls for forged handles that are not input resources.
#[test]
fn test_input_close_rejects_non_input_handle() {
    with_harness_context(|mut context| {
        let forged = context
            .call_context
            .runtime()
            .resources
            .insert(ResourceEntry::new(ResourceKind::Unknown));
        let forged = InputDeviceHandle(forged);

        assert_platform_error_code(
            context.destack_input_close(forged),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return an error when reading one unknown input handle.
#[test]
fn test_input_read_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX));
        assert_platform_error_code(
            context.destack_input_read(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return an error when polling one unknown input handle.
#[test]
fn test_input_try_read_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX - 1));
        assert_platform_error_code(
            context.destack_input_try_read(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return an error when toggling grab on one unknown input handle.
#[test]
fn test_input_set_grab_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX - 2));
        assert_platform_error_code(
            context.destack_input_set_grab(invalid, false),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Poll events non-blocking for one opened input handle when available.
#[test]
fn test_input_try_read_for_available_device() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        if let Some(event) = assert_ok_or_expected_error(
            context.destack_input_try_read(handle),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            let event = context.event_from_value(event)?;
            assert_event_record_semantics(&event);
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Toggle input grab mode when one device handle is available.
#[test]
fn test_input_set_grab_for_available_device() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let _ = assert_ok_or_expected_error(
            context.destack_input_set_grab(handle, false),
            &[
                PlatformErrorCode::NotSupported,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Toggle input grab state and accept host capability outcomes.
#[test]
fn test_input_set_grab_toggle_for_available_device() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        for enable in [true, false] {
            let _ = assert_ok_or_expected_error(
                context.destack_input_set_grab(handle, enable),
                &[
                    PlatformErrorCode::NotSupported,
                    PlatformErrorCode::IoPermissionDenied,
                ],
            )?;
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Report not-supported grab semantics for windows raw input handles.
#[cfg(windows)]
#[test]
fn test_input_set_grab_rejects_windows_raw_handles() {
    with_harness_context(|mut context| {
        for id in ["raw:keyboard", "raw:mouse"] {
            let Some(handle) = assert_ok_or_expected_error(
                context.destack_input_open(context.string_value(id)),
                &[PlatformErrorCode::IoWouldBlock],
            )?
            else {
                continue;
            };
            assert_platform_error_code(
                context.destack_input_set_grab(handle, true),
                PlatformErrorCode::NotSupported,
            )?;
            context.destack_input_close(handle)?;
        }

        Ok(())
    });
}

/// Report not-supported grab semantics for macos global-session handles.
#[cfg(target_os = "macos")]
#[test]
fn test_input_set_grab_rejects_macos_session_handle() {
    with_harness_context(|mut context| {
        let handle = context.destack_input_open(context.string_value("macos:session"))?;
        assert_platform_error_code(
            context.destack_input_set_grab(handle, true),
            PlatformErrorCode::NotSupported,
        )?;
        context.destack_input_close(handle)?;

        Ok(())
    });
}

/// Return io-not-found when reading from one closed input handle.
#[test]
fn test_input_read_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_read(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when try-reading one closed input handle.
#[test]
fn test_input_try_read_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_try_read(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when toggling grab on one closed input handle.
#[test]
fn test_input_set_grab_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_set_grab(handle, false),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when batch-reading from one unknown input handle.
#[test]
fn test_input_read_batch_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX - 3));
        assert_platform_error_code(
            context.destack_input_read_batch(invalid, 4),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject readBatch requests that provide zero max-events.
#[test]
fn test_input_read_batch_rejects_zero_maxevents() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX - 4));
        assert_platform_error_code(
            context.destack_input_read_batch(invalid, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Return io-not-found when batch-reading from one closed input handle.
#[test]
fn test_input_read_batch_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_read_batch(handle, 4),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when setting read mode on one unknown input handle.
#[test]
fn test_input_set_read_mode_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId(u64::MAX - 5));
        assert_platform_error_code(
            context.destack_input_set_read_mode(invalid, InputReadMode::Cooked),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when setting read mode on one closed input handle.
#[test]
fn test_input_set_read_mode_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_set_read_mode(handle, InputReadMode::Raw),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Enforce read-mode capability outcomes from advertised device metadata.
#[test]
fn test_input_set_read_mode_matches_device_capabilities() {
    with_harness_context(|mut context| {
        let Some((handle, supports_raw, supports_text)) =
            open_first_device_with_record_or_skip(&mut context)?
        else {
            return Ok(());
        };

        let raw_result = context.destack_input_set_read_mode(handle, InputReadMode::Raw);
        match raw_result {
            Ok(()) => {
                assert!(
                    supports_raw,
                    "raw read mode should only succeed when supports_raw is true"
                );
            }
            Err(error) => {
                let Some(code) = error.platform_error().map(|platform| platform.code) else {
                    return Err(error);
                };

                if supports_raw {
                    if code != PlatformErrorCode::IoPermissionDenied {
                        return Err(error);
                    }
                } else if code != PlatformErrorCode::NotSupported {
                    return Err(error);
                }
            }
        }

        let cooked_result = context.destack_input_set_read_mode(handle, InputReadMode::Cooked);
        match cooked_result {
            Ok(()) => {
                assert!(
                    supports_text,
                    "cooked read mode should only succeed when supports_text is true"
                );
            }
            Err(error) => {
                let Some(code) = error.platform_error().map(|platform| platform.code) else {
                    return Err(error);
                };

                if supports_text {
                    if code != PlatformErrorCode::IoPermissionDenied {
                        return Err(error);
                    }
                } else if code != PlatformErrorCode::NotSupported {
                    return Err(error);
                }
            }
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Open and close one monitor handle.
#[test]
fn test_input_monitor_open_close_roundtrip() {
    with_harness_context(|mut context| {
        let Some(handle) = assert_ok_or_expected_error(
            context.destack_input_monitor_open(),
            &[PlatformErrorCode::IoWouldBlock],
        )?
        else {
            return Ok(());
        };
        context.destack_input_monitor_close(handle)?;

        Ok(())
    });
}

/// Reject opening duplicate monitor handles on windows while one is active.
#[cfg(windows)]
#[test]
fn test_input_monitor_open_rejects_second_handle() {
    with_harness_context(|mut context| {
        let Some(first) = assert_ok_or_expected_error(
            context.destack_input_monitor_open(),
            &[PlatformErrorCode::IoWouldBlock],
        )?
        else {
            return Ok(());
        };
        assert_platform_error_code(
            context.destack_input_monitor_open(),
            PlatformErrorCode::IoWouldBlock,
        )?;
        context.destack_input_monitor_close(first)?;

        Ok(())
    });
}

/// Return io-not-found for monitor-close on unknown handles.
#[test]
fn test_input_monitor_close_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputMonitorHandle(ResourceId(u64::MAX - 6));
        assert_platform_error_code(
            context.destack_input_monitor_close(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found for monitor-read on unknown handles.
#[test]
fn test_input_monitor_read_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputMonitorHandle(ResourceId(u64::MAX - 7));
        assert_platform_error_code(
            context.destack_input_monitor_read(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found for monitor-try-read on unknown handles.
#[test]
fn test_input_monitor_try_read_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputMonitorHandle(ResourceId(u64::MAX - 8));
        assert_platform_error_code(
            context.destack_input_monitor_try_read(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Poll monitor events nonblocking and accept empty or first-event outcomes.
#[test]
fn test_input_monitor_try_read_on_open_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = assert_ok_or_expected_error(
            context.destack_input_monitor_open(),
            &[PlatformErrorCode::IoWouldBlock],
        )?
        else {
            return Ok(());
        };

        if let Some(event) = assert_ok_or_expected_error(
            context.destack_input_monitor_try_read(handle),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            let event = context.event_from_value(event)?;
            assert_event_record_semantics(&event);
            assert_eq!(event.kind, InputEventKind::Device);
            assert!(
                event.action == InputEventAction::Connect
                    || event.action == InputEventAction::Disconnect,
                "monitor events should be connect or disconnect"
            );
            assert_eq!(
                event.value,
                if event.action == InputEventAction::Connect {
                    1
                } else {
                    0
                },
                "monitor value should match connect and disconnect semantics"
            );
        }

        context.destack_input_monitor_close(handle)?;
        Ok(())
    });
}
