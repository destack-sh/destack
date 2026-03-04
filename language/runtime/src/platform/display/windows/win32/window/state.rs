use super::*;

/// Read one window descriptor snapshot.
pub(crate) unsafe fn window_descriptor(
    context: &BindingCallContext,
    out: *mut WindowDescriptor,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve the binding and enforce owner-thread access
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.descriptor",
    )?;
    {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        ensure_window_thread(&binding, "destack.display.window.descriptor")?;
    }

    // refresh host state before building the descriptor snapshot
    pump_window_messages(context)?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut binding);

    // build the descriptor payload from cached binding fields
    let descriptor = WindowDescriptor {
        backend: DisplayBackend::Win32,
        id: context.store_string(&binding.id),
        title: context.store_string(&binding.title),
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

    // write the descriptor payload
    unsafe {
        *out = descriptor;
    }

    Ok(())
}

/// Read one window state snapshot.
pub(crate) unsafe fn window_state(
    context: &BindingCallContext,
    out: *mut WindowState,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve the binding and enforce owner-thread access
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.state")?;
    {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        ensure_window_thread(&binding, "destack.display.window.state")?;
    }

    // refresh host state before building the state snapshot
    pump_window_messages(context)?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    refresh_window_snapshot(&mut binding);

    // build the state payload from cached binding fields
    let state = WindowState {
        backend: DisplayBackend::Win32,
        position: binding.position,
        size_logical: binding.size_logical,
        size_physical: binding.size_physical,
        scale_factor_milli: binding.scale_factor_milli,
        visibility: binding.visibility,
        display: binding.display,
        focused: binding.focused,
        occlusion: WindowOcclusionState::Unknown,
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

    // write the state payload
    unsafe {
        *out = state;
    }

    Ok(())
}
