use super::{
    InputHarnessContext, assert_ok_or_expected_error, assert_platform_error_code,
    with_harness_context,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::resource::{InputDeviceHandle, ResourceEntry, ResourceId, ResourceKind};

/// Open the first listed input device when the host exposes one accessible endpoint.
fn open_first_device_or_skip(
    context: &mut InputHarnessContext<'_>,
) -> RuntimeResult<Option<InputDeviceHandle>> {
    let devices = context.destack_input_list()?;
    let devices = context.device_records_from_value(devices)?;
    let Some(device) = devices.first() else {
        return Ok(None);
    };

    assert_ok_or_expected_error(
        context.destack_input_open(context.string_value(&device.id)),
        &[
            PlatformErrorCode::IoPermissionDenied,
            PlatformErrorCode::IoNotFound,
        ],
    )
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
            let event = context.event_from_value(event);
            assert_eq!(event.device, handle);
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
