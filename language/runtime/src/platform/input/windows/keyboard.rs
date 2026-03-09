use super::{core as input_core, xinput as xinput_input};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputKeyboardState};
use crate::platform::{PlatformError, core as core_platform, resource};
use crate::runtime::BindingCallContext;

/// Return whether one resolved binding supports keyboard state queries.
fn is_keyboard_capable_backend(resolved: &input_core::WindowsInputResolved) -> bool {
    if resolved.backend == input_core::WindowsInputBackend::Console {
        return true;
    }

    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return false;
        };
        return raw_device.kind == InputDeviceKind::Keyboard;
    }

    false
}

/// Return one stable runtime device identifier for one resolved backend binding.
fn resolved_device_id(
    resolved: &input_core::WindowsInputResolved,
    operation: &'static str,
) -> RuntimeResult<String> {
    if resolved.backend == input_core::WindowsInputBackend::Console {
        return Ok(input_core::WINDOWS_INPUT_DEVICE_ID.to_string());
    }

    if resolved.backend == input_core::WindowsInputBackend::RawDevice {
        let Some(raw_device) = resolved.raw_device.as_ref() else {
            return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
        };
        return Ok(raw_device.id.clone());
    }

    let Some(user_index) = resolved.xinput_user_index else {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    };
    Ok(xinput_input::xinput_device_id(user_index))
}

/// Read one keyboard state snapshot for one opened Windows input handle.
pub(super) fn keyboard_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputKeyboardState> {
    // resolve one backend binding and validate keyboard capability
    let resolved = input_core::resolve_input(binding, handle, operation)?;
    if !is_keyboard_capable_backend(&resolved) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // collect currently pressed virtual-key codes
    let mut pressed_codes = Vec::new();
    for code in 0..=255u32 {
        if input_core::is_key_pressed(code as i32) {
            pressed_codes.push(code);
        }
    }

    // derive stable metadata fields for this snapshot
    let modifiers = input_core::modifier_bits_from_host();
    let sequence = input_core::next_sequence(binding, handle, operation)?;
    let device_id = resolved_device_id(&resolved, operation)?;

    // emit one full keyboard-state payload
    Ok(InputKeyboardState {
        timestamp_ns: input_core::now_timestamp_ns(),
        sequence,
        device_id: binding.store_string(&device_id),
        modifiers,
        pressed_scan_codes: binding.store_array(pressed_codes.clone()),
        pressed_codes: binding.store_array(pressed_codes),
    })
}

/// Read one keyboard state snapshot.
///
/// Return one current keyboard key and modifier snapshot for one opened keyboard-capable device.
/// Snapshot values represent one point-in-time backend state and can change immediately after read.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where keyboard snapshots are unavailable.
/// Uses backend-specific key-state tables from evdev or terminal backends on Unix.
/// Uses console or raw-input key-state paths on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_keyboard_state(
    binding: &BindingCallContext,
    out: *mut InputKeyboardState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // query one keyboard snapshot from the selected backend
    let snapshot = keyboard_state(binding, handle, "destack.input.keyboard.state")?;

    // write snapshot output
    unsafe {
        *out = snapshot;
    }

    Ok(())
}
