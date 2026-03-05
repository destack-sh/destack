use super::*;

/// Read one window descriptor snapshot.
pub(crate) unsafe fn window_descriptor(
    binding: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve the resolved_binding and enforce owner-thread access
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.descriptor",
    )?;
    {
        let resolved_binding = resolved_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ensure_window_thread(&resolved_binding, "destack.display.window.descriptor")?;
    }

    // refresh host state before building the descriptor snapshot
    pump_window_messages(binding)?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut resolved_binding);

    // build the descriptor payload from cached resolved_binding fields
    let descriptor = WindowDescriptor {
        backend: DisplayBackend::Win32,
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

    // write the descriptor payload
    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn window_state(
    binding: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve the resolved_binding and enforce owner-thread access
    let resolved_binding =
        display_resource::resolve_window_binding(binding, window, "destack.display.window.state")?;
    {
        let resolved_binding = resolved_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        ensure_window_thread(&resolved_binding, "destack.display.window.state")?;
    }

    // refresh host state before building the state snapshot
    pump_window_messages(binding)?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut resolved_binding);

    // build the state payload from cached resolved_binding fields
    let state = WindowState {
        backend: DisplayBackend::Win32,
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

    // write the state payload
    unsafe {
        *out = state;
    }

    Ok(())
}
