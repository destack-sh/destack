use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowDescriptor, WindowState};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core;
use crate::platform::display::unix::appkit::resource as display_resource;

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    core_platform::ensure_out(out, "out")?;

    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.descriptor",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let descriptor = core::descriptor_from_host_state(context, &host_state);

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

    let host_state = display_resource::resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.state",
    )?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let state = core::state_from_host_state(&host_state);

    unsafe {
        *out = state;
    }

    Ok(())
}
