use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;
#[cfg(target_os = "macos")]
use super::macos as input_macos;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputKeyboardLayoutInfo, InputKeyboardState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Read one keyboard snapshot for one opened Unix input handle.
fn keyboard_state(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputKeyboardState> {
    // resolve one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // allocate one sequence number for this snapshot read
    let sequence = input_core::next_unix_event_sequence(binding, handle, operation)?;

    // route by backend and host support
    match resolved_binding.backend {
        input_core::UnixInputBackend::UnixTerminal => {
            Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
        }
        input_core::UnixInputBackend::Platform => {
            #[cfg(target_os = "linux")]
            {
                let Some(descriptor) = resolved_binding.descriptor else {
                    return Err(input_core::input_not_found(operation, handle));
                };

                input_linux::keyboard_state_snapshot(
                    binding,
                    descriptor,
                    sequence,
                    &resolved_binding.device_id,
                    operation,
                )
            }

            #[cfg(target_os = "macos")]
            {
                input_macos::keyboard_state_snapshot(binding, sequence, &resolved_binding.device_id)
            }

            #[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
            {
                let _ = (binding, sequence, resolved_binding);
                Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
            }
        }
    }
}

/// Read one keyboard-layout snapshot for one opened Unix input handle.
fn keyboard_layout(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<InputKeyboardLayoutInfo> {
    // validate one opened unix input binding before reporting unsupported
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;
    let _ = resolved_binding;

    Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed())
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

    // query one keyboard-state snapshot
    let snapshot = keyboard_state(binding, handle, "destack.input.keyboard.state")?;

    // write output payload
    unsafe {
        *out = snapshot;
    }

    Ok(())
}

/// Read one keyboard layout snapshot.
pub(crate) unsafe fn destack_input_keyboard_layout(
    binding: &BindingCallContext,
    out: *mut InputKeyboardLayoutInfo,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // read one keyboard-layout snapshot from the selected backend
    let layout = keyboard_layout(binding, handle, "destack.input.keyboard.layout")?;

    // write output payload
    unsafe {
        *out = layout;
    }

    Ok(())
}
