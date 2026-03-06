use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowDescriptor, WindowState};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::resource as display_resource;
use super::{core, runtime};

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.descriptor",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.descriptor")?;
    let descriptor = core::descriptor_from_binding(context, &binding);

    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.state",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    runtime::ensure_window_thread(&binding, "destack.display.window.state")?;
    let state = core::state_from_binding(&binding);

    unsafe {
        *out = state;
    }

    Ok(())
}
