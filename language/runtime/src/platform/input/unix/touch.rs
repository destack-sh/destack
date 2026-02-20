use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputTouchState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened touch-capable unix binding.
fn resolve_touch_binding(
    context: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input binding
    let binding = input_core::resolve_unix_input_binding(context, handle, operation)?;

    // validate one touch-capable device kind
    if !matches!(
        binding.device_kind,
        InputDeviceKind::Touch | InputDeviceKind::Pen
    ) {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    // validate one platform backend with descriptor-backed polling
    if binding.backend != input_core::UnixInputBackend::Platform || binding.descriptor.is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(operation)).boxed());
    }

    Ok(binding)
}

/// Read one touch state snapshot.
///
/// Return one current touch-contact snapshot for one opened touch-capable device.
/// Contact ordering follows backend delivery order.
///
/// # Platform
/// Unix and Windows, with operation-level `notSupported` where touch snapshots are unavailable.
/// Uses backend-specific contact tables from evdev or libinput-style paths on Unix.
/// Uses pointer-contact APIs on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioWouldBlock, notSupported.
///
/// # Security
/// Requires `input.read`.
///
/// # Replay
/// External, recordable.
pub(crate) unsafe fn destack_input_touch_state(
    context: &BindingCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one touch-capable binding
    let binding = resolve_touch_binding(context, handle, "destack.input.touch.state")?;
    let descriptor = binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.touch.state", handle))?;
    let sequence =
        input_core::next_unix_event_sequence(context, handle, "destack.input.touch.state")?;

    // route by host support
    #[cfg(target_os = "linux")]
    {
        let snapshot = input_linux::touch_state_snapshot(
            context,
            descriptor,
            sequence,
            &binding.device_id,
            "destack.input.touch.state",
        )?;

        // write output payload
        unsafe {
            *out = snapshot;
        }

        return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (context, descriptor, sequence);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.touch.state")).boxed())
    }
}
