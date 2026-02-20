use super::{core as input_core, raw as raw_input, xinput as xinput_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::InputGamepadState;
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read one gamepad state snapshot for one opened Windows input handle.
pub(super) fn gamepad_state(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputGamepadState> {
    // resolve one backend binding and route by gamepad implementation lane
    let resolved = input_core::resolve_input(context, handle, operation)?;
    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return Err(input_core::input_not_found(operation, handle));
        };
        return raw_input::gamepad_state_for_raw_input_device(context, raw_device, operation);
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(input_core::input_not_found(operation, handle));
    };
    let player_index = resolved
        .xinput_player_index_override
        .unwrap_or(user_index.saturating_add(1));
    xinput_input::gamepad_state_for_xinput(context, user_index, player_index, operation)
}

/// Set one gamepad light color for one opened Windows input handle.
pub(super) fn gamepad_set_light(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    red: u8,
    green: u8,
    blue: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve one backend binding and route by gamepad implementation lane
    let resolved = input_core::resolve_input(context, handle, operation)?;
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
    let _ = xinput_input::gamepad_state_for_xinput(context, user_index, player_index, operation)?;

    // preserve requested rgb values for future backend light-control routing
    let _requested_color = (red, green, blue);

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
}

/// Set one gamepad player-index override for one opened Windows input handle.
pub(super) fn gamepad_set_player_index(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    player_index: u8,
    operation: &'static str,
) -> RuntimeResult<()> {
    input_core::set_xinput_player_index_override(context, handle, player_index, operation)
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
    gamepad_set_light(
        context,
        handle,
        red,
        green,
        blue,
        "destack.input.gamepad.setLight",
    )
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

    // read one host-backed gamepad state snapshot
    let state = gamepad_state(context, handle, "destack.input.gamepad.state")?;

    // write snapshot output
    unsafe {
        *out = state;
    }

    Ok(())
}
