use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputGamepadState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened gamepad-capable unix binding.
fn resolve_gamepad_binding(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input binding
    let binding = input_core::resolve_unix_input_binding(context, handle, operation)?;

    // validate one gamepad device kind
    if binding.device_kind != InputDeviceKind::Gamepad {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // validate one platform backend with descriptor-backed polling
    if binding.backend != input_core::UnixInputBackend::Platform || binding.descriptor.is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(binding)
}

/// Read one gamepad snapshot for one opened unix handle.
fn gamepad_state(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // resolve one gamepad-capable binding
    let binding = resolve_gamepad_binding(context, handle, operation)?;
    let descriptor = binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found(operation, handle))?;

    // resolve the effective player-index override for this handle
    let player_index = input_core::gamepad_player_index(context, handle, operation)?;

    // route by host support
    #[cfg(target_os = "linux")]
    {
        return input_linux::gamepad_state_snapshot(context, descriptor, player_index, operation);
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (context, descriptor, player_index);
        Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
    }
}

/// Set one gamepad player-index override for one opened unix handle.
fn gamepad_set_player_index(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // validate one gamepad-capable handle before storing the override
    let _binding = resolve_gamepad_binding(context, handle, operation)?;

    input_core::set_gamepad_player_index(context, handle, player_index, operation)
}

/// Set one gamepad light color.
///
/// Apply one rgb light color for one opened gamepad-capable device when supported.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where gamepad light control is unavailable.
/// Uses backend-specific gamepad light-control APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_gamepad_set_light(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
) -> RuntimeResult<()> {
    // validate one gamepad-capable handle before reporting unsupported light control
    let _binding = resolve_gamepad_binding(context, handle, "destack.input.gamepad.setLight")?;
    let _ = (red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.input.gamepad.setLight",
    ))
    .boxed())
}

/// Set one gamepad player index.
///
/// Apply one player index hint for one opened gamepad-capable device.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where player-index assignment is unavailable.
/// Uses backend-specific gamepad player-index assignment APIs.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioPermissionDenied, notSupported.
///
/// # Security
/// Requires `input.control`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_gamepad_set_player_index(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    playerindex: u8,
) -> RuntimeResult<()> {
    gamepad_set_player_index(
        context,
        handle,
        playerindex,
        "destack.input.gamepad.setPlayerIndex",
    )
}

/// Read one gamepad state snapshot.
///
/// Return one full gamepad state snapshot for one opened gamepad-capable device.
/// Snapshot fields mirror backend-standardized gamepad semantics for axes, buttons, touches, and battery metadata.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where gamepad snapshots are unavailable.
/// Uses backend-specific gamepad state APIs with normalized axes, buttons, touch contacts, and battery metadata.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_gamepad_state(
    context: &BindingCallContext,
    out: *mut InputGamepadState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one host-backed gamepad snapshot
    let state = gamepad_state(context, handle, "destack.input.gamepad.state")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}
