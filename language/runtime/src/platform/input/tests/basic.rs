#![allow(dead_code, unused_imports)]

#[cfg(any(windows, target_os = "macos"))]
use super::InputKeyboardStateRecord;
#[cfg(any(windows, target_os = "macos"))]
use super::assert_not_supported_result;
use super::{
    InputDeviceRecord, InputEventRecord, InputEventRecordKind, InputHarnessContext,
    InputMonitorEventRecord, InputMonitorEventRecordKind, assert_ok_or_expected_error,
    assert_platform_error_code, error_code_from_runtime_error, is_not_supported_code,
    with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
#[cfg(any(windows, target_os = "linux"))]
use crate::platform::input::InputTextRange;
use crate::platform::input::validation::{MAX_RAW_HID_BYTES, MAX_READ_BATCH_EVENTS};
use crate::platform::input::{
    InputDeviceCapabilityKind, InputDeviceKind, InputEventAction, InputReadMode, InputTextGeometry,
    InputTextRectangle, InputTextTransform2D, InputWindowTarget,
};
#[cfg(windows)]
use crate::platform::input::{
    InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult, InputPointerGrabMode,
    InputSensorConfig, InputSensorKind, InputTextInputType,
};
#[cfg(target_os = "linux")]
use crate::platform::input::{
    InputHapticEffectParameters, InputHapticEffectType, InputHapticsResult, InputSensorConfig,
    InputTextInputType,
};
#[cfg(windows)]
use crate::platform::resource::InputTextSessionHandle;
use crate::platform::resource::{
    InputDeviceHandle, InputMonitorHandle, ResourceEntry, ResourceId, ResourceKind, WindowHandle,
};
#[cfg(target_os = "linux")]
use crate::tests::platform::assert_not_supported_result;

const ERR_PERMISSION_OR_NOT_SUPPORTED: [PlatformErrorCode; 2] = [
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::NotSupported,
];
#[cfg(windows)]
const ERR_WOULD_BLOCK_PERMISSION_OR_NOT_SUPPORTED: [PlatformErrorCode; 3] = [
    PlatformErrorCode::IoWouldBlock,
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::NotSupported,
];
#[cfg(windows)]
const ERR_WOULD_BLOCK_PERMISSION_INVALID_DATA_OR_NOT_SUPPORTED: [PlatformErrorCode; 4] = [
    PlatformErrorCode::IoWouldBlock,
    PlatformErrorCode::IoPermissionDenied,
    PlatformErrorCode::IoInvalidData,
    PlatformErrorCode::NotSupported,
];

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
) -> RuntimeResult<Option<(InputDeviceHandle, InputDeviceRecord)>> {
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
            return Ok(Some((handle, device)));
        }
    }

    Ok(None)
}

/// Open the first device that exposes one of the requested capability kinds.
#[cfg(target_os = "linux")]
fn open_first_device_with_capabilities_or_skip(
    context: &mut InputHarnessContext<'_>,
    required_capabilities: &[InputDeviceCapabilityKind],
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // enumerate devices and keep the first opened handle that advertises requested capabilities
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
        let Some(handle) = opened else {
            continue;
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        if required_capabilities
            .iter()
            .any(|required| capabilities.kinds.contains(required))
        {
            return Ok(Some(handle));
        }

        context.destack_input_close(handle)?;
    }

    Ok(None)
}

/// Open the first Linux device that reports raw-hid capability.
#[cfg(target_os = "linux")]
fn open_first_linux_raw_hid_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // enumerate devices and keep the first opened handle that reports raw-hid support
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
        let Some(handle) = opened else {
            continue;
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        if capabilities.supports_raw_hid {
            return Ok(Some(handle));
        }

        context.destack_input_close(handle)?;
    }

    Ok(None)
}

/// Open one windows raw-input device and return metadata when available.
#[cfg(windows)]
fn open_first_windows_raw_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<(InputDeviceHandle, InputDeviceRecord)>> {
    // enumerate devices and select one per-device raw-input endpoint
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    for device in devices {
        if !device.id.starts_with("raw:device:") {
            continue;
        }

        let opened = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value(&device.id)),
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        if let Some(handle) = opened {
            return Ok(Some((handle, device)));
        }
    }

    Ok(None)
}

/// Open the windows console input endpoint when available.
#[cfg(windows)]
fn open_windows_console_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    assert_ok_or_expected_error(
        context.destack_input_open(context.string_value("console:stdin")),
        &[
            PlatformErrorCode::IoPermissionDenied,
            PlatformErrorCode::IoNotFound,
            PlatformErrorCode::IoWouldBlock,
        ],
    )
}

/// Open the first xinput device when one connected controller is available.
#[cfg(windows)]
fn open_first_windows_xinput_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    for device in devices {
        if !device.id.starts_with("xinput:") {
            continue;
        }

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

/// Open the first windows raw-input device with one of the requested capability kinds.
#[cfg(windows)]
fn open_first_windows_raw_device_with_capabilities_or_skip(
    context: &mut InputHarnessContext<'_>,
    required_capabilities: &[InputDeviceCapabilityKind],
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // enumerate windows raw-input endpoints and inspect runtime capability metadata
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    for device in devices {
        if !device.id.starts_with("raw:device:") {
            continue;
        }

        let opened = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value(&device.id)),
            &[
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoNotFound,
                PlatformErrorCode::IoWouldBlock,
            ],
        )?;
        let Some(handle) = opened else {
            continue;
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        let has_required_capability = required_capabilities
            .iter()
            .any(|required| capabilities.kinds.contains(required));
        if has_required_capability {
            return Ok(Some(handle));
        }

        context.destack_input_close(handle)?;
    }

    Ok(None)
}

/// Open the first windows raw gamepad endpoint when one is available.
#[cfg(windows)]
fn open_first_windows_raw_gamepad_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    open_first_windows_raw_device_with_capabilities_or_skip(
        context,
        &[InputDeviceCapabilityKind::Gamepad],
    )
}

/// Open the first windows touch or pen input endpoint when one is available.
#[cfg(windows)]
fn open_first_windows_touch_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    open_first_windows_raw_device_with_capabilities_or_skip(
        context,
        &[
            InputDeviceCapabilityKind::Touch,
            InputDeviceCapabilityKind::Pen,
        ],
    )
}

/// Open the first windows sensor input endpoint when one is available.
#[cfg(windows)]
fn open_first_windows_sensor_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    open_first_windows_raw_device_with_capabilities_or_skip(
        context,
        &[InputDeviceCapabilityKind::Sensor],
    )
}

/// Open one windows keyboard-capable device and fall back to console when available.
#[cfg(windows)]
fn open_first_windows_keyboard_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // prefer the console path because raw-input keyboards do not yet expose per-device snapshots
    open_windows_console_device_or_skip(context)
}

/// Open one windows pointer-capable device and fall back to console when available.
#[cfg(windows)]
fn open_first_windows_pointer_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    // prefer per-device raw pointer endpoints first
    let raw = open_first_windows_raw_device_with_capabilities_or_skip(
        context,
        &[InputDeviceCapabilityKind::Pointer],
    )?;
    if raw.is_some() {
        return Ok(raw);
    }

    // fall back to console pointer stream
    open_windows_console_device_or_skip(context)
}

/// Build one default input target that uses process focus scope.
#[cfg(windows)]
fn default_input_target() -> InputWindowTarget {
    InputWindowTarget {
        window: Some(WindowHandle(ResourceId::local(0))),
    }
}

/// Build one explicit input target that requires window-scoped routing.
#[cfg(windows)]
fn explicit_input_target() -> InputWindowTarget {
    InputWindowTarget {
        window: Some(WindowHandle(ResourceId::local(1))),
    }
}

/// Create one opened window resource for input target tests.
#[cfg(any(windows, target_os = "linux"))]
fn register_input_target_window(context: &mut InputHarnessContext<'_>) -> WindowHandle {
    let entry = ResourceEntry::new(ResourceKind::Window);

    #[cfg(windows)]
    let entry = entry.with_handle(1usize as *mut c_void);

    let resource_id = context.call_context.worker().resources.insert(
        context.call_context.world(),
        entry,
        Some(context.call_context.engine()),
    );

    WindowHandle(resource_id)
}

/// Assert one decoded event payload uses valid runtime fields.
fn assert_event_record_shape(event: &InputEventRecord) {
    assert!(
        !event.device_id.is_empty(),
        "event device id should not be empty"
    );
    assert!(
        event.timestamp_ns > 0,
        "event timestamp should be monotonic and nonzero"
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
        InputEventRecordKind::Key => {
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
        InputEventRecordKind::PointerMotion => {
            assert_eq!(
                event.action,
                InputEventAction::Move,
                "pointer motion should use move action"
            );
        }
        InputEventRecordKind::PointerButton => {
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
        InputEventRecordKind::Scroll => {
            assert_eq!(
                event.action,
                InputEventAction::Scroll,
                "scroll events should use scroll action"
            );
        }
        InputEventRecordKind::Touch => {
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
        InputEventRecordKind::Gamepad | InputEventRecordKind::Sensor => {}
        InputEventRecordKind::Text => {
            assert_eq!(
                event.action,
                InputEventAction::Text,
                "text events should use text action"
            );
        }
        InputEventRecordKind::Device => {
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
        InputEventRecordKind::Composition => {
            assert!(
                matches!(
                    event.action,
                    InputEventAction::Begin
                        | InputEventAction::Update
                        | InputEventAction::Commit
                        | InputEventAction::End
                        | InputEventAction::Cancel
                ),
                "composition events should use composition actions"
            );
        }
    }
}

/// Assert one decoded monitor payload uses coherent topology semantics.
fn assert_monitor_event_record_semantics(event: &InputMonitorEventRecord) {
    assert!(
        !event.device_id.is_empty(),
        "monitor device id should not be empty"
    );
    assert!(
        event.timestamp_ns > 0,
        "monitor timestamp should be monotonic and nonzero"
    );
    assert!(
        event.sequence > 0,
        "monitor sequence should be monotonic and nonzero"
    );
    assert!(
        matches!(
            event.kind,
            InputMonitorEventRecordKind::Connect
                | InputMonitorEventRecordKind::Disconnect
                | InputMonitorEventRecordKind::Change
        ),
        "monitor events should use monitor event kinds"
    );
}

/// Assert one decoded keyboard snapshot uses coherent runtime fields.
#[cfg(any(windows, target_os = "macos"))]
fn assert_keyboard_state_record_semantics(state: &InputKeyboardStateRecord) {
    assert!(
        !state.device_id.is_empty(),
        "keyboard snapshot device id should not be empty"
    );
    assert!(
        state.timestamp_ns > 0,
        "keyboard snapshot timestamp should be monotonic and nonzero"
    );
    assert!(
        state.sequence > 0,
        "keyboard snapshot sequence should be monotonic and nonzero"
    );
    assert!(
        state.pressed_code_count <= 256,
        "keyboard snapshot pressed key count should stay in vk range"
    );
    assert!(
        state.pressed_scan_code_count <= 256,
        "keyboard snapshot pressed scan-code count should stay in vk range"
    );
}

/// Exercise keyboard snapshots on the macos global-session endpoint.
#[cfg(target_os = "macos")]
#[test]
fn test_input_macos_keyboard_state_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let handle = context.destack_input_open(context.string_value("macos:session"))?;
        let state = context.destack_input_keyboard_state(handle)?;
        let state = context.keyboard_state_from_value(state)?;

        assert_keyboard_state_record_semantics(&state);
        assert_eq!(
            state.device_id, "macos:session",
            "macos keyboard snapshots should report the session device id"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise pointer snapshots and relative-mode toggles on the macos global-session endpoint.
#[cfg(target_os = "macos")]
#[test]
fn test_input_macos_pointer_state_and_relative_mode_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let handle = context.destack_input_open(context.string_value("macos:session"))?;

        let absolute = context.destack_input_pointer_state(handle)?;
        let absolute = context.pointer_state_from_value(absolute);
        assert!(
            absolute.x.is_finite() && absolute.y.is_finite(),
            "macos absolute pointer snapshots should use finite coordinates"
        );

        context.destack_input_pointer_set_relative_mode(handle, true)?;
        let relative = context.destack_input_pointer_relative_state(handle)?;
        let relative = context.pointer_state_from_value(relative);
        assert!(
            relative.x.is_finite() && relative.y.is_finite(),
            "macos relative pointer snapshots should use finite deltas"
        );
        context.destack_input_pointer_set_relative_mode(handle, false)?;

        let target = InputWindowTarget {
            window: Some(WindowHandle(ResourceId::local(0))),
        };
        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(target),
            true,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(target),
            false,
        ))?;

        let _ = assert_ok_or_expected_error(
            context.destack_input_pointer_warp(handle, context.window_target(target), 4.0, 4.0),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Keep window-target harness conversion available for target-scoped API tests.
#[test]
fn test_input_harness_window_target_helper_roundtrip() {
    with_harness_context(|context| {
        let target = InputWindowTarget {
            window: Some(WindowHandle(ResourceId::local(1))),
        };
        let _ = context.window_target(target);

        Ok(())
    });
}

/// Keep text-area harness conversion available for text-session API tests.
#[test]
fn test_input_harness_text_input_area_helper_roundtrip() {
    with_harness_context(|context| {
        let area = InputTextGeometry {
            local_to_target_transform: InputTextTransform2D {
                xx: 1.0,
                xy: 0.0,
                yx: 0.0,
                yy: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            editor_rectangle: InputTextRectangle {
                x: 12.0,
                y: 34.0,
                width: 567.0,
                height: 890.0,
            },
            caret_rectangle: Some(InputTextRectangle {
                x: 22.0,
                y: 44.0,
                width: 2.0,
                height: 18.0,
            }),
            composing_rectangle: None,
        };
        let value = context.text_input_area(area);
        let decoded = context.text_input_area_from(value);
        assert_eq!(decoded, area);

        Ok(())
    });
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

/// Open and close per-device windows raw-input identifiers.
#[cfg(windows)]
#[test]
fn test_input_open_close_roundtrip_for_windows_raw_devices() {
    with_harness_context(|mut context| {
        let devices = context.destack_input_list()?;
        let devices = context.device_records_from_value(devices)?;
        let mut opened_count = 0usize;

        for device in devices {
            if !device.id.starts_with("raw:device:") {
                continue;
            }

            let Some(handle) = assert_ok_or_expected_error(
                context.destack_input_open(context.string_value(&device.id)),
                &[
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::IoNotFound,
                    PlatformErrorCode::IoWouldBlock,
                ],
            )?
            else {
                continue;
            };
            context.destack_input_close(handle)?;
            opened_count += 1;
        }

        let _ = opened_count;
        Ok(())
    });
}

/// Reject opening duplicate windows console handles while one stream is active.
#[cfg(windows)]
#[test]
fn test_input_open_rejects_second_windows_console_handle() {
    with_harness_context(|mut context| {
        let Some(first) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_input_open(context.string_value("console:stdin")),
            PlatformErrorCode::IoWouldBlock,
        )?;
        context.destack_input_close(first)?;

        Ok(())
    });
}

/// Reject opening duplicate windows raw-input handles for one same device id.
#[cfg(windows)]
#[test]
fn test_input_open_rejects_second_windows_raw_handle_for_same_device() {
    with_harness_context(|mut context| {
        let Some((first, device)) = open_first_windows_raw_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_input_open(context.string_value(&device.id)),
            PlatformErrorCode::IoWouldBlock,
        )?;
        context.destack_input_close(first)?;

        Ok(())
    });
}

/// Report honest exclusive-grab metadata for windows raw-input devices.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_device_metadata_does_not_claim_exclusive_grab() {
    with_harness_context(|mut context| {
        let Some((_handle, device)) = open_first_windows_raw_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert!(
            !device.supports_exclusive_grab,
            "windows raw-input devices should not claim exclusive-grab support"
        );

        Ok(())
    });
}

/// Open distinct windows raw-input handles or report stream contention when busy.
#[cfg(windows)]
#[test]
fn test_input_open_distinct_windows_raw_handles_or_reports_contention() {
    with_harness_context(|mut context| {
        let Some((first_handle, first_device)) =
            open_first_windows_raw_device_or_skip(&mut context)?
        else {
            return Ok(());
        };

        let devices = context.destack_input_list()?;
        let devices = context.device_records_from_value(devices)?;
        let second_device = devices
            .into_iter()
            .find(|device| device.id.starts_with("raw:device:") && device.id != first_device.id);
        let Some(second_device) = second_device else {
            context.destack_input_close(first_handle)?;
            return Ok(());
        };

        let second = assert_ok_or_expected_error(
            context.destack_input_open(context.string_value(&second_device.id)),
            &[PlatformErrorCode::IoWouldBlock],
        )?;
        if let Some(second_handle) = second {
            context.destack_input_close(second_handle)?;
        }
        context.destack_input_close(first_handle)?;

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

/// Report keyboard and pointer capabilities for the macos global-session backend.
#[cfg(target_os = "macos")]
#[test]
fn test_input_capabilities_for_macos_session_report_keyboard_and_pointer() {
    with_harness_context(|mut context| {
        let handle = context.destack_input_open(context.string_value("macos:session"))?;
        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;

        assert!(
            capabilities
                .kinds
                .contains(&InputDeviceCapabilityKind::Keyboard),
            "macos session capabilities should include keyboard kind"
        );
        assert!(
            capabilities
                .kinds
                .contains(&InputDeviceCapabilityKind::Pointer),
            "macos session capabilities should include pointer kind"
        );
        assert!(
            capabilities.button_codes.len() >= 5,
            "macos session should expose primary pointer buttons"
        );
        assert!(
            capabilities.axis_codes.len() >= 4,
            "macos session should expose pointer and wheel axes"
        );
        assert!(
            capabilities.supports_pointer_warp,
            "macos session capabilities should report pointer warp support"
        );
        assert!(
            !capabilities.supports_pointer_capture,
            "macos session capabilities should report pointer capture as unsupported"
        );

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

/// Keep linux event payload ids aligned with listed stable device ids.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_event_device_id_matches_listed_device_id() {
    with_harness_context(|mut context| {
        let Some((handle, device)) = open_first_device_with_record_or_skip(&mut context)? else {
            return Ok(());
        };

        // accept empty queues and permission-gated hosts while enforcing id coherence when events exist
        if let Some(event) = assert_ok_or_expected_error(
            context.destack_input_try_read(handle),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::NotSupported,
            ],
        )? {
            let event = context.event_from_value(event)?;
            assert_eq!(
                event.device_id, device.id,
                "linux events should report the same stable device id returned by input.list/open"
            );
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Emit stable runtime ids for Linux host input devices.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_list_uses_stable_runtime_ids() {
    with_harness_context(|mut context| {
        let devices = context.destack_input_list()?;
        let devices = context.device_records_from_value(devices)?;
        for device in devices {
            if device.id == "tty:stdin" {
                continue;
            }

            assert!(
                device.id.starts_with("linux:evdev:") || device.id.starts_with("linux:hidraw:"),
                "linux device ids should use stable runtime prefixes: got {}",
                device.id
            );
        }

        Ok(())
    });
}

/// Reject close calls for forged handles that are not input resources.
#[test]
fn test_input_close_rejects_non_input_handle() {
    with_harness_context(|mut context| {
        let forged = context.call_context.worker().resources.insert(
            context.call_context.world(),
            ResourceEntry::new(ResourceKind::File),
            Some(context.call_context.engine()),
        );
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
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX));
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
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 1));
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
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 2));
        assert_platform_error_code(
            context.destack_input_set_exclusive_grab(invalid, false),
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

        assert_ok_or_expected_error(
            context.destack_input_set_exclusive_grab(handle, false),
            &ERR_PERMISSION_OR_NOT_SUPPORTED,
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
            assert_ok_or_expected_error(
                context.destack_input_set_exclusive_grab(handle, enable),
                &ERR_PERMISSION_OR_NOT_SUPPORTED,
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
        let devices = context.destack_input_list()?;
        let devices = context.device_records_from_value(devices)?;
        for device in devices {
            if !device.id.starts_with("raw:device:") {
                continue;
            }

            let Some(handle) = assert_ok_or_expected_error(
                context.destack_input_open(context.string_value(&device.id)),
                &[
                    PlatformErrorCode::IoPermissionDenied,
                    PlatformErrorCode::IoNotFound,
                    PlatformErrorCode::IoWouldBlock,
                ],
            )?
            else {
                continue;
            };
            assert_not_supported_result(context.destack_input_set_exclusive_grab(handle, true))?;
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
        assert_not_supported_result(context.destack_input_set_exclusive_grab(handle, true))?;
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
            context.destack_input_set_exclusive_grab(handle, false),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when batch-reading from one unknown input handle.
#[test]
fn test_input_read_batch_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 3));
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
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 4));
        assert_platform_error_code(
            context.destack_input_read_batch(invalid, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject readBatch requests that exceed one bounded batch-size limit.
#[test]
fn test_input_read_batch_rejects_excessive_maxevents() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 34));
        assert_platform_error_code(
            context.destack_input_read_batch(invalid, MAX_READ_BATCH_EVENTS + 1),
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
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 5));
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
        let Some((handle, device)) = open_first_device_with_record_or_skip(&mut context)? else {
            return Ok(());
        };

        let raw_result = context.destack_input_set_read_mode(handle, InputReadMode::Raw);
        match raw_result {
            Ok(()) => {
                assert!(
                    device.supports_raw,
                    "raw read mode should only succeed when supports_raw is true"
                );
            }
            Err(error) => {
                let Some(code) = error_code_from_runtime_error(&error) else {
                    return Err(error);
                };

                if device.supports_raw {
                    if code != PlatformErrorCode::IoPermissionDenied {
                        return Err(error);
                    }
                } else if !is_not_supported_code(Some(code)) {
                    return Err(error);
                }
            }
        }

        let cooked_result = context.destack_input_set_read_mode(handle, InputReadMode::Cooked);
        match cooked_result {
            Ok(()) => {
                assert!(
                    device.supports_text,
                    "cooked read mode should only succeed when supports_text is true"
                );
            }
            Err(error) => {
                let Some(code) = error_code_from_runtime_error(&error) else {
                    return Err(error);
                };

                if device.supports_text {
                    if code != PlatformErrorCode::IoPermissionDenied {
                        return Err(error);
                    }
                } else if !is_not_supported_code(Some(code)) {
                    return Err(error);
                }
            }
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Return io-not-found when querying capabilities on one unknown input handle.
#[test]
fn test_input_capabilities_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 9));
        assert_platform_error_code(
            context.destack_input_capabilities(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Return io-not-found when querying capabilities on one closed input handle.
#[test]
fn test_input_capabilities_rejects_closed_handle() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_or_skip(&mut context)? else {
            return Ok(());
        };

        context.destack_input_close(handle)?;
        assert_platform_error_code(
            context.destack_input_capabilities(handle),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Report coherent capabilities payload fields for one opened input device.
#[test]
fn test_input_capabilities_match_device_metadata() {
    with_harness_context(|mut context| {
        let Some((handle, device)) = open_first_device_with_record_or_skip(&mut context)? else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;

        if device.kind == InputDeviceKind::Gamepad {
            assert!(
                capabilities
                    .kinds
                    .contains(&InputDeviceCapabilityKind::Gamepad),
                "gamepad devices should expose gamepad capability kind"
            );
        }
        if device.kind == InputDeviceKind::Keyboard {
            assert!(
                capabilities
                    .kinds
                    .contains(&InputDeviceCapabilityKind::Keyboard),
                "keyboard devices should expose keyboard capability kind"
            );
        }
        if matches!(device.kind, InputDeviceKind::Mouse | InputDeviceKind::Pen) {
            assert!(
                capabilities
                    .kinds
                    .contains(&InputDeviceCapabilityKind::Pointer),
                "pointer devices should expose pointer capability kind"
            );
        }
        if device.supports_text {
            assert!(
                capabilities.supports_text_input,
                "text-capable devices should report supports_text_input"
            );
            assert!(
                capabilities
                    .kinds
                    .contains(&InputDeviceCapabilityKind::TextInput),
                "text-capable devices should include textInput capability kind"
            );
        }
        if device.supports_rumble {
            assert!(
                capabilities.supports_rumble,
                "rumble-capable devices should report supports_rumble"
            );
            assert!(
                capabilities
                    .kinds
                    .contains(&InputDeviceCapabilityKind::Haptics),
                "rumble-capable devices should include haptics capability kind"
            );
        }
        if device.button_count > 0 {
            assert!(
                !capabilities.button_codes.is_empty(),
                "devices with button_count should expose button capability codes"
            );
        }
        if device.axis_count > 0 {
            assert!(
                !capabilities.axis_codes.is_empty(),
                "devices with axis_count should expose axis capability codes"
            );
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
        let invalid = InputMonitorHandle(ResourceId::local(u64::MAX - 6));
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
        let invalid = InputMonitorHandle(ResourceId::local(u64::MAX - 7));
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
        let invalid = InputMonitorHandle(ResourceId::local(u64::MAX - 8));
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
            let event = context.monitor_event_from_value(event)?;
            assert_monitor_event_record_semantics(&event);
        }

        context.destack_input_monitor_close(handle)?;
        Ok(())
    });
}

/// Reject unknown explicit window targets on windows pointer APIs.
#[cfg(windows)]
#[test]
fn test_input_windows_pointer_target_rejects_explicit_window() {
    with_harness_context(|mut context| {
        let Some(handle) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_input_pointer_warp(
                handle,
                context.window_target(explicit_input_target()),
                0.0,
                0.0,
            ),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_capture(
                handle,
                context.window_target(explicit_input_target()),
                true,
            ),
            PlatformErrorCode::IoNotFound,
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Accept default-target pointer capture toggles for windows console handles.
#[cfg(windows)]
#[test]
fn test_input_windows_pointer_capture_accepts_default_target() {
    with_harness_context(|mut context| {
        let Some(handle) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_ok_or_expected_error(
            context.destack_input_pointer_capture(
                handle,
                context.window_target(default_input_target()),
                true,
            ),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;
        assert_ok_or_expected_error(
            context.destack_input_pointer_capture(
                handle,
                context.window_target(default_input_target()),
                false,
            ),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Report not-supported pointer capture on Linux evdev-style pointer handles.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_pointer_capture_reports_not_supported() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Pointer],
        )?
        else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        assert!(
            !capabilities.supports_pointer_capture,
            "linux pointer capabilities should not claim pointer capture support"
        );

        let target = InputWindowTarget {
            window: Some(WindowHandle(ResourceId::local(0))),
        };
        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(target),
            true,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(target),
            false,
        ))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise pointer snapshots and relative-mode toggles for one Linux pointer-capable handle.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_pointer_state_and_relative_mode_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Pointer],
        )?
        else {
            return Ok(());
        };

        let absolute = context.destack_input_pointer_state(handle)?;
        let absolute = context.pointer_state_from_value(absolute);
        assert!(
            absolute.x.is_finite() && absolute.y.is_finite(),
            "absolute pointer snapshots should use finite coordinates"
        );
        assert!(
            absolute
                .pen
                .map(|pen| pen.pressure.is_finite())
                .unwrap_or(true),
            "absolute pen pressure should be finite when available"
        );

        assert_not_supported_result(context.destack_input_pointer_relative_state(handle))?;
        assert_ok_or_expected_error(
            context.destack_input_pointer_set_relative_mode(handle, true),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        let relative = context.destack_input_pointer_relative_state(handle)?;
        let relative = context.pointer_state_from_value(relative);
        assert!(
            relative.x.is_finite() && relative.y.is_finite(),
            "relative pointer snapshots should use finite deltas"
        );

        assert_ok_or_expected_error(
            context.destack_input_pointer_set_relative_mode(handle, false),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Accept explicit window targets on windows text APIs.
#[cfg(windows)]
#[test]
fn test_input_windows_text_target_accepts_explicit_window() {
    with_harness_context(|mut context| {
        let target_window = register_input_target_window(&mut context);
        let session = context.destack_input_text_open(
            context.text_session_config(
                InputWindowTarget {
                    window: Some(target_window),
                },
                InputTextInputType::Text,
                false,
                false,
            ),
            context.text_session_state(
                "",
                InputTextRange {
                    start_offset: 0,
                    end_offset: 0,
                },
                None,
            ),
        )?;
        context.destack_input_text_close(session)?;

        Ok(())
    });
}

/// Report would-block when windows text sessions have no pending committed text.
#[cfg(windows)]
#[test]
fn test_input_windows_text_try_read_event_reports_would_block_without_text() {
    with_harness_context(|mut context| {
        let session = context.destack_input_text_open(
            context.text_session_config(
                default_input_target(),
                InputTextInputType::Text,
                false,
                false,
            ),
            context.text_session_state(
                "",
                InputTextRange {
                    start_offset: 0,
                    end_offset: 0,
                },
                None,
            ),
        )?;

        assert_platform_error_code(
            context.destack_input_text_try_read_event(session),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_input_text_close(session)?;
        Ok(())
    });
}

/// Accept explicit window targets on Unix text APIs.
#[cfg(target_os = "linux")]
#[test]
fn test_input_unix_text_target_accepts_explicit_window() {
    with_harness_context(|mut context| {
        let target_window = register_input_target_window(&mut context);
        let session = context.destack_input_text_open(
            context.text_session_config(
                InputWindowTarget {
                    window: Some(target_window),
                },
                InputTextInputType::Text,
                false,
                false,
            ),
            context.text_session_state(
                "",
                InputTextRange {
                    start_offset: 0,
                    end_offset: 0,
                },
                None,
            ),
        )?;

        assert_platform_error_code(
            context.destack_input_text_get_geometry(session),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_input_text_close(session)?;
        Ok(())
    });
}

/// Report would-block when Unix text sessions have no pending committed text.
#[cfg(target_os = "linux")]
#[test]
fn test_input_unix_text_try_read_event_reports_would_block_without_text() {
    with_harness_context(|mut context| {
        let session = context.destack_input_text_open(
            context.text_session_config(
                InputWindowTarget { window: None },
                InputTextInputType::Text,
                false,
                false,
            ),
            context.text_session_state(
                "",
                InputTextRange {
                    start_offset: 0,
                    end_offset: 0,
                },
                None,
            ),
        )?;

        assert_platform_error_code(
            context.destack_input_text_try_read_event(session),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_input_text_close(session)?;
        Ok(())
    });
}

/// Report truthful pointer and text capability booleans on windows console handles.
#[cfg(windows)]
#[test]
fn test_input_windows_console_capabilities_report_supported_pointer_features() {
    with_harness_context(|mut context| {
        let Some(handle) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        assert!(
            capabilities.supports_relative_pointer,
            "console backend should report relative pointer support"
        );
        assert!(
            !capabilities.supports_pointer_grab,
            "console backend should not claim pointer-grab support without a real lock or confine primitive"
        );
        assert!(
            !capabilities.supports_pointer_capture,
            "console backend should not claim pointer capture support without real capture semantics"
        );
        assert!(
            capabilities.supports_pointer_warp,
            "console backend should report pointer warp support"
        );
        assert!(
            !capabilities.supports_composition,
            "console backend should not claim composition support without real IME lifecycle events"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Reject out-of-range windows xinput player index overrides.
#[cfg(windows)]
#[test]
fn test_input_windows_gamepad_player_index_rejects_out_of_range() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_xinput_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_input_gamepad_set_player_index(handle, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_input_gamepad_set_player_index(handle, 5),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise windows raw-gamepad snapshots with capability-aware assertions.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_gamepad_state_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_raw_gamepad_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        assert!(
            capabilities
                .kinds
                .contains(&InputDeviceCapabilityKind::Gamepad),
            "raw gamepad handles should expose gamepad capability kind"
        );

        let state = context.destack_input_gamepad_state(handle)?;
        let state = context.gamepad_state_from_value(state)?;
        assert!(
            state.connected,
            "raw gamepad snapshots should report connected state"
        );
        assert_eq!(
            state.axis_count, 4,
            "raw gamepad snapshots should project standard axis slots"
        );
        assert_eq!(
            state.button_count, 17,
            "raw gamepad snapshots should project standard button slots"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Return io-not-found for gamepad light-control requests with unknown handles.
#[cfg(windows)]
#[test]
fn test_input_windows_gamepad_set_light_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 21));
        assert_platform_error_code(
            context.destack_input_gamepad_set_light(invalid, 1, 2, 3),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject zero max-byte budgets on windows raw-hid read APIs.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_hid_rejects_zero_maxbytes() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 22));

        assert_platform_error_code(
            context.destack_input_raw_hid_read(invalid, 0, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code(
            context.destack_input_raw_hid_try_read(invalid, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code(
            context.destack_input_raw_hid_get_feature(invalid, 0, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Reject max-byte budgets that exceed one bounded raw-hid payload limit.
#[test]
fn test_input_raw_hid_rejects_excessive_maxbytes() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 35));
        let excessive_maxbytes = MAX_RAW_HID_BYTES + 1;

        assert_platform_error_code(
            context.destack_input_raw_hid_read(invalid, excessive_maxbytes, 0),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code(
            context.destack_input_raw_hid_try_read(invalid, excessive_maxbytes),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        assert_platform_error_code(
            context.destack_input_raw_hid_get_feature(invalid, 0, excessive_maxbytes),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        Ok(())
    });
}

/// Exercise windows raw-hid calls with capability-aware assertions.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_hid_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some((handle, _device)) = open_first_windows_raw_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        if !capabilities.supports_raw_hid {
            assert_not_supported_result(context.destack_input_raw_hid_try_read(handle, 64))?;
            assert_not_supported_result(context.destack_input_raw_hid_get_feature(handle, 0, 64))?;

            let payload = context.bytes_value(&[0])?;
            assert_not_supported_result(
                context.destack_input_raw_hid_set_feature(handle, 0, payload),
            )?;
            let payload = context.bytes_value(&[0])?;
            assert_not_supported_result(context.destack_input_raw_hid_write(handle, 0, payload))?;

            context.destack_input_close(handle)?;
            return Ok(());
        }

        if let Some(report) = assert_ok_or_expected_error(
            context.destack_input_raw_hid_try_read(handle, 64),
            &ERR_WOULD_BLOCK_PERMISSION_OR_NOT_SUPPORTED,
        )? {
            let report = context.raw_hid_report_from_value(report)?;
            assert!(
                report.sequence > 0,
                "raw-hid report sequence should be nonzero"
            );
            assert!(
                report.data.len() <= 64,
                "raw-hid report bytes should honor max-byte budget"
            );
        }

        if let Some(report) = assert_ok_or_expected_error(
            context.destack_input_raw_hid_read(handle, 64, 1_000_000),
            &ERR_WOULD_BLOCK_PERMISSION_OR_NOT_SUPPORTED,
        )? {
            let report = context.raw_hid_report_from_value(report)?;
            assert!(
                report.data.len() <= 64,
                "timed raw-hid reads should honor max-byte budget"
            );
        }

        if let Some(feature) = assert_ok_or_expected_error(
            context.destack_input_raw_hid_get_feature(handle, 0, 64),
            &ERR_WOULD_BLOCK_PERMISSION_INVALID_DATA_OR_NOT_SUPPORTED,
        )? {
            let bytes = context.bytes_from_value(feature)?;
            assert!(
                bytes.len() <= 64,
                "feature-report bytes should honor max-byte budget"
            );
        }

        let payload = context.bytes_value(&[0])?;
        assert_ok_or_expected_error(
            context.destack_input_raw_hid_set_feature(handle, 0, payload),
            &ERR_WOULD_BLOCK_PERMISSION_INVALID_DATA_OR_NOT_SUPPORTED,
        )?;

        let payload = context.bytes_value(&[0])?;
        assert_ok_or_expected_error(
            context.destack_input_raw_hid_write(handle, 0, payload),
            &ERR_WOULD_BLOCK_PERMISSION_INVALID_DATA_OR_NOT_SUPPORTED,
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise Linux raw-hid bindings with capability-aware assertions.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_raw_hid_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_linux_raw_hid_device_or_skip(&mut context)? else {
            return Ok(());
        };

        if let Some(report) = assert_ok_or_expected_error(
            context.destack_input_raw_hid_try_read(handle, 64),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            let report = context.raw_hid_report_from_value(report)?;
            assert!(
                report.data.len() <= 64,
                "raw-hid report bytes should honor max-byte budget"
            );
        }

        if let Some(feature) = assert_ok_or_expected_error(
            context.destack_input_raw_hid_get_feature(handle, 0, 64),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoInvalidData,
            ],
        )? {
            let bytes = context.bytes_from_value(feature)?;
            assert!(
                bytes.len() <= 64,
                "feature-report bytes should honor max-byte budget"
            );
        }

        let payload = context.bytes_value(&[0])?;
        assert_ok_or_expected_error(
            context.destack_input_raw_hid_set_feature(handle, 0, payload),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoInvalidData,
            ],
        )?;

        let payload = context.bytes_value(&[0])?;
        assert_ok_or_expected_error(
            context.destack_input_raw_hid_write(handle, 0, payload),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoPermissionDenied,
                PlatformErrorCode::IoInvalidData,
            ],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise windows touch-state reads with capability-aware assertions.
#[cfg(windows)]
#[test]
fn test_input_windows_touch_state_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_touch_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let state = context.destack_input_touch_state(handle)?;
        let state = context.touch_state_from_value(state)?;
        assert!(
            !state.device_id.is_empty(),
            "touch snapshots should include a stable device id"
        );
        assert!(
            state.sequence > 0,
            "touch snapshots should include a nonzero sequence"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise windows sensor bindings with capability-aware assertions.
#[cfg(windows)]
#[test]
fn test_input_windows_sensor_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_sensor_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let sensors = context.destack_input_sensor_list(handle)?;
        let sensors = context.sensor_infos_from_value(sensors)?;
        assert!(
            !sensors.is_empty(),
            "sensor-capable handles should list at least one sensor lane"
        );

        let kind = sensors[0].kind;
        let config = InputSensorConfig {
            enabled: true,
            sample_rate_hz: 60.0,
            batch_latency_ms: 0,
            flags: 0,
        };
        let effective =
            context.destack_input_sensor_configure(handle, kind, context.sensor_config(config))?;
        let effective = context.sensor_effective_config_from_value(effective);
        assert!(
            effective.sample_rate_hz >= 0.0,
            "effective sensor sample rate should be non-negative"
        );

        assert_platform_error_code(
            context.destack_input_sensor_configure(
                handle,
                kind,
                context.sensor_config(InputSensorConfig {
                    enabled: true,
                    sample_rate_hz: 0.0,
                    batch_latency_ms: 0,
                    flags: 0,
                }),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        if let Some(sample) = assert_ok_or_expected_error(
            context.destack_input_sensor_try_read(handle, kind),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            let sample = context.sensor_sample_from_value(sample);
            assert_eq!(
                sample.kind, kind,
                "sensor sample kind should match requested sensor lane"
            );
        }

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise Linux sensor bindings with capability-aware assertions.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_sensor_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Sensor],
        )?
        else {
            return Ok(());
        };

        let sensors = context.destack_input_sensor_list(handle)?;
        let sensors = context.sensor_infos_from_value(sensors)?;
        assert!(
            !sensors.is_empty(),
            "sensor-capable handles should list at least one sensor lane"
        );

        let kind = sensors[0].kind;
        let config = InputSensorConfig {
            enabled: true,
            sample_rate_hz: 60.0,
            batch_latency_ms: 0,
            flags: 0,
        };
        let effective =
            context.destack_input_sensor_configure(handle, kind, context.sensor_config(config))?;
        let effective = context.sensor_effective_config_from_value(effective);
        assert!(
            effective.enabled,
            "enabled sensor configuration should report one enabled effective config"
        );

        assert_platform_error_code(
            context.destack_input_sensor_configure(
                handle,
                kind,
                context.sensor_config(InputSensorConfig {
                    enabled: true,
                    sample_rate_hz: 0.0,
                    batch_latency_ms: 0,
                    flags: 0,
                }),
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        if let Some(sample) = assert_ok_or_expected_error(
            context.destack_input_sensor_try_read(handle, kind),
            &[PlatformErrorCode::IoWouldBlock],
        )? {
            let sample = context.sensor_sample_from_value(sample);
            assert_eq!(
                sample.kind, kind,
                "sensor sample kind should match requested sensor lane"
            );
        }

        let disabled_config = InputSensorConfig {
            enabled: false,
            sample_rate_hz: 0.0,
            batch_latency_ms: 0,
            flags: 0,
        };
        let _ = context.destack_input_sensor_configure(
            handle,
            kind,
            context.sensor_config(disabled_config),
        )?;
        assert_platform_error_code(
            context.destack_input_sensor_try_read(handle, kind),
            PlatformErrorCode::IoWouldBlock,
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Return io-not-found for sensor-read calls with unknown handles.
#[cfg(windows)]
#[test]
fn test_input_windows_sensor_read_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 23));
        assert_platform_error_code(
            context.destack_input_sensor_read(invalid, InputSensorKind::Accelerometer),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Reject unknown handles across the extended windows input surface.
#[cfg(windows)]
#[test]
fn test_input_windows_extended_surface_rejects_unknown_handle() {
    with_harness_context(|mut context| {
        let invalid = InputDeviceHandle(ResourceId::local(u64::MAX - 24));
        let invalid_text_session = InputTextSessionHandle(ResourceId::local(u64::MAX - 25));
        let target = context.window_target(default_input_target());

        assert_platform_error_code(
            context.destack_input_keyboard_state(invalid),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_state(invalid),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_relative_state(invalid),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_set_relative_mode(invalid, true),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_set_grab_mode(
                invalid,
                target,
                InputPointerGrabMode::None,
            ),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_text_get_geometry(invalid_text_session),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_haptics_effects(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        let params = InputHapticEffectParameters {
            duration_ms: 16,
            start_delay_ms: 0,
            strong_magnitude: 0.5,
            weak_magnitude: 0.25,
            left_trigger: 0.0,
            right_trigger: 0.0,
        };
        assert_platform_error_code(
            context.destack_input_haptics_play(
                invalid,
                InputHapticEffectType::DualRumble,
                context.haptics_parameters(params),
            ),
            PlatformErrorCode::IoNotFound,
        )?;
        assert_platform_error_code(
            context.destack_input_haptics_stop(invalid),
            PlatformErrorCode::IoNotFound,
        )?;

        Ok(())
    });
}

/// Exercise keyboard snapshot reads for one windows keyboard-capable handle.
#[cfg(windows)]
#[test]
fn test_input_windows_keyboard_state_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_keyboard_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let state = context.destack_input_keyboard_state(handle)?;
        let state = context.keyboard_state_from_value(state)?;
        assert_keyboard_state_record_semantics(&state);
        assert_eq!(
            state.pressed_scan_code_count, 0,
            "windows console keyboard snapshots should leave scan-code arrays empty until one real scan-code path exists"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Report not-supported raw keyboard snapshots on windows until per-device state exists.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_keyboard_state_reports_not_supported() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_raw_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Keyboard],
        )?
        else {
            return Ok(());
        };

        assert_not_supported_result(context.destack_input_keyboard_state(handle))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise pointer snapshots and relative-mode toggles for one windows pointer handle.
#[cfg(windows)]
#[test]
fn test_input_windows_pointer_state_and_relative_mode_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_pointer_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let absolute = context.destack_input_pointer_state(handle)?;
        let absolute = context.pointer_state_from_value(absolute);
        assert!(
            absolute.x.is_finite() && absolute.y.is_finite(),
            "absolute pointer snapshots should use finite coordinates"
        );
        assert!(
            absolute
                .pen
                .map(|pen| pen.pressure.is_finite())
                .unwrap_or(true),
            "absolute pen pressure should be finite when available"
        );

        assert_not_supported_result(context.destack_input_pointer_relative_state(handle))?;

        assert_ok_or_expected_error(
            context.destack_input_pointer_set_relative_mode(handle, true),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;
        if let Some(relative) = assert_ok_or_expected_error(
            context.destack_input_pointer_relative_state(handle),
            &[
                PlatformErrorCode::IoWouldBlock,
                PlatformErrorCode::IoPermissionDenied,
            ],
        )? {
            let relative = context.pointer_state_from_value(relative);
            assert!(
                relative.x.is_finite() && relative.y.is_finite(),
                "relative pointer snapshots should use finite deltas"
            );
        }
        assert_ok_or_expected_error(
            context.destack_input_pointer_set_relative_mode(handle, false),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise pointer grab-mode transitions for one windows raw pointer-capable handle.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_pointer_set_grab_mode_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_raw_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Pointer],
        )?
        else {
            return Ok(());
        };

        context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::None,
        )?;
        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Locked,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Confined,
        ))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Reject target-less locked grabs on windows raw pointer endpoints.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_pointer_locked_grab_requires_explicit_target() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_raw_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Pointer],
        )?
        else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        assert!(
            capabilities.supports_pointer_grab,
            "windows raw pointer capabilities should only claim pointer-grab support when one real locked or confined path exists"
        );

        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Locked,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Confined,
        ))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Report unsupported pointer capture and grab semantics on the windows console backend.
#[cfg(windows)]
#[test]
fn test_input_windows_console_pointer_capture_and_grab_report_not_supported() {
    with_harness_context(|mut context| {
        let Some(handle) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(default_input_target()),
            true,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Locked,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_set_grab_mode(
            handle,
            context.window_target(default_input_target()),
            InputPointerGrabMode::Confined,
        ))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Reject non-finite pointer-warp coordinates on windows pointer-capable handles.
#[cfg(windows)]
#[test]
fn test_input_windows_pointer_warp_rejects_non_finite_coordinates() {
    with_harness_context(|mut context| {
        let Some(handle) = open_windows_console_device_or_skip(&mut context)? else {
            return Ok(());
        };

        assert_platform_error_code(
            context.destack_input_pointer_warp(
                handle,
                context.window_target(default_input_target()),
                f64::NAN,
                0.0,
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;
        assert_platform_error_code(
            context.destack_input_pointer_warp(
                handle,
                context.window_target(default_input_target()),
                0.0,
                f64::INFINITY,
            ),
            PlatformErrorCode::InvalidArgumentValue,
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Report not-supported pointer capture for windows raw pointer handles.
#[cfg(windows)]
#[test]
fn test_input_windows_raw_pointer_capture_reports_not_supported() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_raw_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Pointer],
        )?
        else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        assert!(
            !capabilities.supports_pointer_capture,
            "windows raw pointer capabilities should report pointer capture as unsupported"
        );

        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(default_input_target()),
            true,
        ))?;
        assert_not_supported_result(context.destack_input_pointer_capture(
            handle,
            context.window_target(explicit_input_target()),
            true,
        ))?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise text session lifecycle and state roundtrips on windows.
#[cfg(windows)]
#[test]
fn test_input_windows_text_session_tracks_lifecycle() {
    with_harness_context(|mut context| {
        let session = context.destack_input_text_open(
            context.text_session_config(
                default_input_target(),
                InputTextInputType::Text,
                false,
                false,
            ),
            context.text_session_state(
                "hello",
                InputTextRange {
                    start_offset: 0,
                    end_offset: 5,
                },
                None,
            ),
        )?;

        assert_platform_error_code(
            context.destack_input_text_get_geometry(session),
            PlatformErrorCode::IoWouldBlock,
        )?;

        let area = InputTextGeometry {
            local_to_target_transform: InputTextTransform2D {
                xx: 1.0,
                xy: 0.0,
                yx: 0.0,
                yy: 1.0,
                tx: 0.0,
                ty: 0.0,
            },
            editor_rectangle: InputTextRectangle {
                x: 32.0,
                y: 48.0,
                width: 512.0,
                height: 256.0,
            },
            caret_rectangle: Some(InputTextRectangle {
                x: 37.0,
                y: 52.0,
                width: 2.0,
                height: 18.0,
            }),
            composing_rectangle: None,
        };
        context.destack_input_text_set_geometry(session, context.text_input_area(area))?;
        let current_area = context.destack_input_text_get_geometry(session)?;
        let current_area = context.text_input_area_from(current_area);
        assert_eq!(
            current_area, area,
            "text area get/set should roundtrip for windows text sessions"
        );

        context.destack_input_text_set_state(
            session,
            context.text_session_state(
                "hello world",
                InputTextRange {
                    start_offset: 6,
                    end_offset: 11,
                },
                Some(InputTextRange {
                    start_offset: 0,
                    end_offset: 5,
                }),
            ),
        )?;

        context.destack_input_text_close(session)?;
        assert_platform_error_code(
            context.destack_input_text_get_geometry(session),
            PlatformErrorCode::IoNotFound,
        );

        Ok(())
    });
}

/// Exercise haptics effect listing and playback on windows xinput endpoints when available.
#[cfg(windows)]
#[test]
fn test_input_windows_haptics_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_xinput_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let capabilities = context.destack_input_capabilities(handle)?;
        let capabilities = context.capabilities_from_value(capabilities)?;
        if !capabilities.supports_rumble
            || !capabilities
                .kinds
                .contains(&InputDeviceCapabilityKind::Haptics)
        {
            assert_not_supported_result(context.destack_input_haptics_effects(handle))?;

            let params = InputHapticEffectParameters {
                duration_ms: 16,
                start_delay_ms: 0,
                strong_magnitude: 0.7,
                weak_magnitude: 0.3,
                left_trigger: 0.2,
                right_trigger: 0.2,
            };
            assert_not_supported_result(context.destack_input_haptics_play(
                handle,
                InputHapticEffectType::DualRumble,
                context.haptics_parameters(params),
            ))?;
            assert_not_supported_result(context.destack_input_haptics_stop(handle))?;

            context.destack_input_close(handle)?;
            return Ok(());
        }

        let Some(effects) = assert_ok_or_expected_error(
            context.destack_input_haptics_effects(handle),
            &[PlatformErrorCode::IoNotFound],
        )?
        else {
            context.destack_input_close(handle)?;
            return Ok(());
        };
        let effects = context.haptic_effects_from_value(effects)?;
        assert!(
            !effects.is_empty(),
            "xinput haptics-capable endpoints should report at least one effect kind"
        );

        let effect = effects[0];
        let params = InputHapticEffectParameters {
            duration_ms: 16,
            start_delay_ms: 0,
            strong_magnitude: 0.7,
            weak_magnitude: 0.3,
            left_trigger: 0.2,
            right_trigger: 0.2,
        };
        if let Some(result) = assert_ok_or_expected_error(
            context.destack_input_haptics_play(handle, effect, context.haptics_parameters(params)),
            &[PlatformErrorCode::IoNotFound],
        )? {
            assert!(
                matches!(
                    result,
                    InputHapticsResult::Complete | InputHapticsResult::Preempted
                ),
                "haptics playback should report a concrete completion state"
            );
        }
        let _ = assert_ok_or_expected_error(
            context.destack_input_haptics_stop(handle),
            &[PlatformErrorCode::IoNotFound],
        )?;

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Reject out-of-range haptics parameters on windows xinput endpoints.
#[cfg(windows)]
#[test]
fn test_input_windows_haptics_play_rejects_out_of_range_params() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_windows_xinput_device_or_skip(&mut context)? else {
            return Ok(());
        };

        let params = InputHapticEffectParameters {
            duration_ms: 16,
            start_delay_ms: 0,
            strong_magnitude: 1.25,
            weak_magnitude: 0.3,
            left_trigger: 0.0,
            right_trigger: 0.0,
        };
        let result = assert_ok_or_expected_error(
            context.destack_input_haptics_play(
                handle,
                InputHapticEffectType::DualRumble,
                context.haptics_parameters(params),
            ),
            &[
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoNotFound,
            ],
        )?;
        assert!(
            result.is_none(),
            "out-of-range haptics magnitudes should fail instead of silently succeeding"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Exercise Linux haptics effect listing and playback when one rumble endpoint is available.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_haptics_surface_matches_capabilities() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Haptics],
        )?
        else {
            return Ok(());
        };

        let effects = context.destack_input_haptics_effects(handle)?;
        let effects = context.haptic_effects_from_value(effects)?;
        assert!(
            !effects.is_empty(),
            "haptics-capable endpoints should report at least one effect kind"
        );

        let effect = effects[0];
        assert_eq!(
            effect,
            InputHapticEffectType::DualRumble,
            "linux haptics should expose dual-rumble effect lanes"
        );

        let params = InputHapticEffectParameters {
            duration_ms: 16,
            start_delay_ms: 0,
            strong_magnitude: 0.7,
            weak_magnitude: 0.3,
            left_trigger: 0.0,
            right_trigger: 0.0,
        };
        if let Some(result) = assert_ok_or_expected_error(
            context.destack_input_haptics_play(handle, effect, context.haptics_parameters(params)),
            &[PlatformErrorCode::IoPermissionDenied],
        )? {
            assert!(
                matches!(
                    result,
                    InputHapticsResult::Complete | InputHapticsResult::Preempted
                ),
                "haptics playback should report one concrete completion state"
            );
        }

        let _ = assert_ok_or_expected_error(
            context.destack_input_haptics_stop(handle),
            &[PlatformErrorCode::IoPermissionDenied],
        )?;
        context.destack_input_close(handle)?;
        Ok(())
    });
}

/// Reject out-of-range haptics parameters on Linux rumble endpoints.
#[cfg(target_os = "linux")]
#[test]
fn test_input_linux_haptics_play_rejects_out_of_range_params() {
    with_harness_context(|mut context| {
        let Some(handle) = open_first_device_with_capabilities_or_skip(
            &mut context,
            &[InputDeviceCapabilityKind::Haptics],
        )?
        else {
            return Ok(());
        };

        let params = InputHapticEffectParameters {
            duration_ms: 16,
            start_delay_ms: 0,
            strong_magnitude: 1.25,
            weak_magnitude: 0.3,
            left_trigger: 0.0,
            right_trigger: 0.0,
        };
        let result = assert_ok_or_expected_error(
            context.destack_input_haptics_play(
                handle,
                InputHapticEffectType::DualRumble,
                context.haptics_parameters(params),
            ),
            &[
                PlatformErrorCode::InvalidArgument,
                PlatformErrorCode::IoNotFound,
            ],
        )?;
        assert!(
            result.is_none(),
            "out-of-range haptics magnitudes should fail instead of silently succeeding"
        );

        context.destack_input_close(handle)?;
        Ok(())
    });
}

// FUGU #Cleanup: split and organize input/tests/basic.rs properly (also see other modules)
#[cfg(windows)]
use std::ffi::c_void;
