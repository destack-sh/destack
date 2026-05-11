use super::{core as input_core, raw as raw_input, xinput as xinput_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::InputGamepadState;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read one gamepad state snapshot for one opened Windows input handle.
pub(super) fn gamepad_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // resolve one backend binding and route by gamepad implementation lane
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return Err(input_core::input_not_found(operation, handle));
        };
        return raw_input::gamepad_state_for_raw_input_device(binding, raw_device, operation);
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    let player_index = resolved
        .xinput_player_index_override
        .unwrap_or(user_index.saturating_add(1));
    xinput_input::gamepad_state_for_xinput(binding, user_index, player_index, operation)
}

/// Set one gamepad light color for one opened Windows input handle.
pub(super) fn gamepad_set_light(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and route by gamepad implementation lane
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return Err(input_core::input_not_found(operation, handle));
        };
        return raw_input::set_gamepad_light_for_raw_input_device(
            raw_device, red, green, blue, operation,
        );
    }

    // validate one connected xinput endpoint before reporting unsupported light control
    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    let player_index = resolved
        .xinput_player_index_override
        .unwrap_or(user_index.saturating_add(1));
    let _ = xinput_input::gamepad_state_for_xinput(binding, user_index, player_index, operation)?;

    // preserve requested rgb values for future backend light-control routing
    let _requested_color = (red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Set one gamepad player-index override for one opened Windows input handle.
pub(super) fn gamepad_set_player_index(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    input_core::set_xinput_player_index_override(binding, handle, player_index, operation)
}

/// Validate one gamepad motion-sensor operation for one opened Windows input handle.
pub(super) fn gamepad_validate_motion_sensors(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding before reporting unsupported motion sensors
    let resolved = input_core::resolve_input(binding, handle, operation)?;

    // validate one routed gamepad endpoint shape
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return Err(input_core::input_not_found(operation, handle));
        };
        let _ = raw_device;
    } else {
        let Some(user_index) = resolved.xinput_user_index else {
            return Err(input_core::input_not_found(operation, handle));
        };
        let player_index = resolved
            .xinput_player_index_override
            .unwrap_or(user_index.saturating_add(1));
        let _ =
            xinput_input::gamepad_state_for_xinput(binding, user_index, player_index, operation)?;
    }

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Set one gamepad light color.
pub(crate) unsafe fn destack_input_gamepad_set_light(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    gamepad_set_light(
        binding,
        handle,
        red,
        green,
        blue,
        "destack.input.gamepad.setLight",
    )
}

/// Set one gamepad motion sensor sample rate.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensor_sample_rate(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    sampleratehz: f64,
) -> RuntimeResult<()> {
    let _ = sampleratehz;

    gamepad_validate_motion_sensors(
        binding,
        handle,
        "destack.input.gamepad.setMotionSensorSampleRate",
    )
}

/// Enable or disable one gamepad motion sensor stream.
pub(crate) unsafe fn destack_input_gamepad_set_motion_sensors_enabled(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    enabled: bool,
) -> RuntimeResult<()> {
    let _ = enabled;

    gamepad_validate_motion_sensors(
        binding,
        handle,
        "destack.input.gamepad.setMotionSensorsEnabled",
    )
}

/// Set one gamepad player index.
pub(crate) unsafe fn destack_input_gamepad_set_player_index(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    gamepad_set_player_index(
        binding,
        handle,
        playerindex,
        "destack.input.gamepad.setPlayerIndex",
    )
}

/// Read one gamepad state snapshot.
pub(crate) unsafe fn destack_input_gamepad_state(
    binding: &BindingCallContext,
    out: *mut InputGamepadState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one host-backed gamepad state snapshot
    let state = gamepad_state(binding, handle, "destack.input.gamepad.state")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}
