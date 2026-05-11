use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputGamepadState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened gamepad-capable unix binding.
fn resolve_gamepad_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // validate one gamepad device kind
    if resolved_binding.device_kind != InputDeviceKind::Gamepad {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // validate one platform backend with descriptor-backed polling
    if resolved_binding.backend != input_core::UnixInputBackend::Platform
        || resolved_binding.descriptor.is_none()
    {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(resolved_binding)
}

/// Read one gamepad snapshot for one opened unix handle.
fn gamepad_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // resolve one gamepad-capable resolved_binding
    let resolved_binding = resolve_gamepad_binding(binding, handle, operation)?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found(operation, handle))?;

    // resolve the effective player-index override for this handle
    let player_index = input_core::gamepad_player_index(binding, handle, operation)?;

    // route by host support
    #[cfg(target_os = "linux")]
    {
        input_linux::gamepad_state_snapshot(binding, descriptor, player_index, operation)
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, descriptor, player_index);
        Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
    }
}

/// Set one gamepad player-index override for one opened unix handle.
fn gamepad_set_player_index(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate one gamepad-capable handle before storing the override
    let _binding = resolve_gamepad_binding(binding, handle, operation)?;

    input_core::set_gamepad_player_index(binding, handle, player_index, operation)
}

/// Validate one gamepad motion-sensor operation for one opened unix handle.
fn gamepad_validate_motion_sensors(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate one gamepad-capable handle before reporting unsupported
    let _binding = resolve_gamepad_binding(binding, handle, operation)?;

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
    // validate one gamepad-capable handle before reporting unsupported light control
    let _binding = resolve_gamepad_binding(binding, handle, "destack.input.gamepad.setLight")?;
    let _ = (red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setLight",
    ))
    .boxed())
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

    // read one host-backed gamepad snapshot
    let state = gamepad_state(binding, handle, "destack.input.gamepad.state")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}
