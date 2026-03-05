use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    DisplayBackend, WindowDescriptor, WindowOcclusionState, WindowState,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::resource as display_resource;

/// Read descriptor metadata for one window.
pub(crate) unsafe fn window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and refresh host event state
    core_platform::ensure_out(out, "out")?;
    super::pump_window_messages(binding)?;

    // resolve one resolved_binding snapshot and encode descriptor payload
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.descriptor",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.descriptor")?;
    let descriptor = WindowDescriptor {
        backend: DisplayBackend::X11,
        id: binding.store_string(&resolved_binding.id),
        title: binding.store_string(&resolved_binding.title),
        mode: resolved_binding.mode,
        display: resolved_binding.display,
        resizable: resolved_binding.resizable,
        decorated: resolved_binding.decorated,
        chrome: resolved_binding.chrome,
        taskbar_visible: resolved_binding.taskbar_visible,
        transparent: resolved_binding.transparent,
        opacity: resolved_binding.opacity,
        always_on_top: resolved_binding.always_on_top,
        parent: resolved_binding.parent,
        transient_for: resolved_binding.transient_for,
        modal: resolved_binding.modal,
        mouse_passthrough: resolved_binding.mouse_passthrough,
        aspect_ratio: resolved_binding.aspect_ratio,
    };
    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer and refresh host event state
    core_platform::ensure_out(out, "out")?;
    super::pump_window_messages(binding)?;

    // resolve one resolved_binding snapshot and encode state payload
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window_handle,
        "destack.display.window.state",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&resolved_binding, "destack.display.window.state")?;
    let state = WindowState {
        backend: DisplayBackend::X11,
        position: resolved_binding.position,
        size_logical: resolved_binding.size_logical,
        size_physical: resolved_binding.size_physical,
        scale_factor_milli: resolved_binding.scale_factor_milli,
        visibility: resolved_binding.visibility,
        display: resolved_binding.display,
        focused: resolved_binding.focused,
        occlusion: WindowOcclusionState::Unknown,
        safe_area_insets: resolved_binding.safe_area_insets,
        theme: resolved_binding.theme,
        chrome: resolved_binding.chrome,
        taskbar_visible: resolved_binding.taskbar_visible,
        opacity: resolved_binding.opacity,
        always_on_top: resolved_binding.always_on_top,
        parent: resolved_binding.parent,
        transient_for: resolved_binding.transient_for,
        modal: resolved_binding.modal,
        mouse_passthrough: resolved_binding.mouse_passthrough,
        aspect_ratio: resolved_binding.aspect_ratio,
    };
    unsafe {
        *out = state;
    }

    Ok(())
}
