use super::core as input_core;
#[cfg(target_os = "linux")]
use super::linux as input_linux;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::input::{InputDeviceKind, InputTouchState};
use crate::platform::{PlatformError, resource};
use crate::runtime::BindingCallContext;

/// Resolve one opened touch-capable unix binding.
fn resolve_touch_binding(
    binding: &BindingCallContext,
    handle: resource::InputDeviceHandle,
    operation: &'static str,
) -> RuntimeResult<input_core::UnixInputBinding> {
    // resolve one opened unix input resolved_binding
    let resolved_binding = input_core::resolve_unix_input_binding(binding, handle, operation)?;

    // validate one touch-capable device kind
    if !matches!(
        resolved_binding.device_kind,
        InputDeviceKind::Touch | InputDeviceKind::Pen
    ) {
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

/// Read one touch state snapshot.
pub(crate) unsafe fn destack_input_touch_state(
    binding: &BindingCallContext,
    out: *mut InputTouchState,
    handle: resource::InputDeviceHandle,
) -> RuntimeResult<()> {
    // validate output pointer
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }

    // resolve one touch-capable resolved_binding
    let resolved_binding = resolve_touch_binding(binding, handle, "destack.input.touch.state")?;
    let descriptor = resolved_binding
        .descriptor
        .ok_or_else(|| input_core::input_not_found("destack.input.touch.state", handle))?;
    let sequence =
        input_core::next_unix_event_sequence(binding, handle, "destack.input.touch.state")?;

    // route by host support
    #[cfg(target_os = "linux")]
    {
        let snapshot = input_linux::touch_state_snapshot(
            binding,
            descriptor,
            sequence,
            &resolved_binding.device_id,
            "destack.input.touch.state",
        )?;

        // write output payload
        unsafe {
            *out = snapshot;
        }

        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = (binding, descriptor, sequence);
        Err(RuntimeError::from(PlatformError::not_supported("destack.input.touch.state")).boxed())
    }
}
