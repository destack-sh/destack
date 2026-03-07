use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowDescriptor, WindowState};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{occlusion_from_visibility, resolve_window_host_state};
use crate::platform::display::unix::wayland::core as wayland_core;

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer before encoding one descriptor snapshot
    core_platform::ensure_out(out, "out")?;

    // resolve one host_state snapshot and encode descriptor payload
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.descriptor")?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let descriptor = WindowDescriptor {
        backend: wayland_core::selected_backend(),
        id: context.store_string(&host_state.id),
        title: context.store_string(&host_state.title),
        role: host_state.role,
        mode: host_state.mode,
        display: host_state.display,
        resizable: host_state.resizable,
        decorated: host_state.decorated,
        chrome: host_state.chrome,
        taskbar_visible: host_state.taskbar_visible,
        transparent: host_state.transparent,
        opacity: host_state.opacity,
        always_on_top: host_state.always_on_top,
        parent: host_state.parent,
        transient_for: host_state.transient_for,
        modal: host_state.modal,
        mouse_passthrough: host_state.mouse_passthrough,
        aspect_ratio: host_state.aspect_ratio,
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
    // validate out pointer before encoding one state snapshot
    core_platform::ensure_out(out, "out")?;

    // resolve one host_state snapshot and encode state payload
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.state")?;
    let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    let state = WindowState {
        backend: wayland_core::selected_backend(),
        position: host_state.position,
        size_logical: host_state.size_logical,
        size_physical: host_state.size_physical,
        scale_factor_milli: host_state.scale_factor_milli,
        visibility: host_state.visibility,
        role: host_state.role,
        display: host_state.display,
        focused: host_state.focused,
        occlusion: occlusion_from_visibility(host_state.visibility),
        safe_area_insets: host_state.safe_area_insets,
        theme: host_state.theme,
        chrome: host_state.chrome,
        taskbar_visible: host_state.taskbar_visible,
        opacity: host_state.opacity,
        always_on_top: host_state.always_on_top,
        parent: host_state.parent,
        transient_for: host_state.transient_for,
        modal: host_state.modal,
        mouse_passthrough: host_state.mouse_passthrough,
        aspect_ratio: host_state.aspect_ratio,
    };

    unsafe {
        *out = state;
    }

    Ok(())
}
