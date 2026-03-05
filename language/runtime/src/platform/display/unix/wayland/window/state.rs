use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowDescriptor, WindowState};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::core as backend_core;

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and refresh backend window state
    core_platform::ensure_out(out, "out")?;
    super::pump_window_messages(context)?;

    // resolve one binding snapshot and encode descriptor payload
    let binding =
        super::resolve_window_binding(context, window_handle, "destack.display.window.descriptor")?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    let _host_id = binding.host.id;

    let descriptor = WindowDescriptor {
        backend: backend_core::selected_backend(),
        id: context.store_string(&binding.id),
        title: context.store_string(&binding.title),
        role: binding.role,
        mode: binding.mode,
        display: binding.display,
        resizable: binding.resizable,
        decorated: binding.decorated,
        chrome: binding.chrome,
        taskbar_visible: binding.taskbar_visible,
        transparent: binding.transparent,
        opacity: binding.opacity,
        always_on_top: binding.always_on_top,
        parent: binding.parent,
        transient_for: binding.transient_for,
        modal: binding.modal,
        mouse_passthrough: binding.mouse_passthrough,
        aspect_ratio: binding.aspect_ratio,
    };

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
    // validate out pointer and refresh backend window state
    core_platform::ensure_out(out, "out")?;
    super::pump_window_messages(context)?;

    // resolve one binding snapshot and encode state payload
    let binding =
        super::resolve_window_binding(context, window_handle, "destack.display.window.state")?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    let state = WindowState {
        backend: backend_core::selected_backend(),
        position: binding.position,
        size_logical: binding.size_logical,
        size_physical: binding.size_physical,
        scale_factor_milli: binding.scale_factor_milli,
        visibility: binding.visibility,
        role: binding.role,
        display: binding.display,
        focused: binding.focused,
        occlusion: super::occlusion_from_visibility(binding.visibility),
        safe_area_insets: binding.safe_area_insets,
        theme: binding.theme,
        chrome: binding.chrome,
        taskbar_visible: binding.taskbar_visible,
        opacity: binding.opacity,
        always_on_top: binding.always_on_top,
        parent: binding.parent,
        transient_for: binding.transient_for,
        modal: binding.modal,
        mouse_passthrough: binding.mouse_passthrough,
        aspect_ratio: binding.aspect_ratio,
    };

    unsafe {
        *out = state;
    }

    Ok(())
}
