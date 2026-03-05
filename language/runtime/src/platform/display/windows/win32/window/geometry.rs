use super::*;

/// Apply one optional aspect-ratio lock to one logical-size payload.
fn apply_aspect_ratio_lock(
    size: WindowLogicalSize,
    aspect_ratio: Option<WindowAspectRatio>,
) -> WindowLogicalSize {
    // return original size when no lock is configured
    let Some(aspect_ratio) = aspect_ratio else {
        return size;
    };

    // resolve ratio and clamp adjusted height
    let numerator = aspect_ratio.numerator as f64;
    let denominator = aspect_ratio.denominator as f64;
    let adjusted_height = ((size.width * denominator) / numerator).max(1.0);
    WindowLogicalSize {
        width: size.width,
        height: adjusted_height,
    }
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setPosition",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setPosition")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // apply host position update
    let status = unsafe {
        SetWindowPos(
            resolved_binding.hwnd,
            0,
            position.x,
            position.y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setPosition",
            "SetWindowPos",
            "failed to set window position",
        ));
    }

    // refresh cached state and publish deltas
    resolved_binding.position = position;
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    // validate logical size payload
    let size = normalize_logical_size(size, "size")?;

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setSizeLogical",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setSizeLogical")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // resolve constrained target size and host outer rectangle
    let locked_size = apply_aspect_ratio_lock(size, resolved_binding.aspect_ratio);
    let clamped_size = clamp_logical_size(locked_size, resolved_binding.constraints);
    let size_physical = logical_to_physical(clamped_size, resolved_binding.scale_factor_milli);
    let style = window_style_for_binding(&resolved_binding);
    let ex_style = window_ex_style_for_binding(&resolved_binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        resolved_binding.hwnd,
        size_physical,
        style,
        ex_style,
        "destack.display.window.setSizeLogical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            resolved_binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizeLogical",
            "SetWindowPos",
            "failed to set logical window size",
        ));
    }

    // refresh cached state and publish deltas
    resolved_binding.size_logical = clamped_size;
    resolved_binding.size_physical = size_physical;
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    // validate physical size payload
    let size = normalize_physical_size(size, "size")?;

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setSizePhysical",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setSizePhysical")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // resolve constrained target size and host outer rectangle
    let size_logical = physical_to_logical(size, resolved_binding.scale_factor_milli);
    let size_logical = apply_aspect_ratio_lock(size_logical, resolved_binding.aspect_ratio);
    let clamped_logical = clamp_logical_size(size_logical, resolved_binding.constraints);
    let clamped_physical =
        logical_to_physical(clamped_logical, resolved_binding.scale_factor_milli);
    let style = window_style_for_binding(&resolved_binding);
    let ex_style = window_ex_style_for_binding(&resolved_binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        resolved_binding.hwnd,
        clamped_physical,
        style,
        ex_style,
        "destack.display.window.setSizePhysical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            resolved_binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizePhysical",
            "SetWindowPos",
            "failed to set physical window size",
        ));
    }

    // refresh cached state and publish deltas
    resolved_binding.size_logical = clamped_logical;
    resolved_binding.size_physical = clamped_physical;
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setSizeConstraints",
    )?;

    // update cached constraints
    resolved_binding.constraints = constraints;

    Ok(())
}

/// Set one window mode payload.
pub(crate) unsafe fn window_set_mode(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setMode",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setMode")?;

    // short circuit no-op mode transitions
    if same_window_mode(resolved_binding.mode, mode) {
        return Ok(());
    }

    // apply transition and rollback in-memory state on host failure
    let previous = resolved_binding.clone();
    // evaluate this condition
    if let Err(error) = apply_mode_options(
        binding,
        &mut resolved_binding,
        mode,
        "destack.display.window.setMode",
        true,
    ) {
        let _ = apply_mode_options(
            binding,
            &mut resolved_binding,
            previous.mode,
            "destack.display.window.setMode.rollback",
            false,
        );
        resolved_binding.mode = previous.mode;
        resolved_binding.display = previous.display;
        resolved_binding.exclusive_restore = previous.exclusive_restore.clone();
        refresh_window_snapshot(&mut resolved_binding);

        return Err(error);
    }

    // refresh cached state and publish semantic deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    // evaluate this condition
    if !same_window_mode(previous.mode, next.mode) {
        event::publish_window_mode_event(&event_runtime_state, window, previous.mode, next.mode);
    }
    // evaluate this condition
    if previous.display != next.display {
        event::publish_window_display_event(
            &event_runtime_state,
            window,
            previous.display,
            next.display,
        );
    }
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn window_set_aspect_ratio(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    aspectratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    // validate non-zero ratio payload
    if let Some(aspectratio) = aspectratio
        && (aspectratio.numerator == 0 || aspectratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must be greater than zero",
        ));
    }

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setAspectRatio",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setAspectRatio")?;

    // capture previous state and apply aspect ratio update
    let previous = resolved_binding.clone();
    resolved_binding.aspect_ratio = aspectratio;

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.minimize",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.minimize")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // apply host minimize transition
    unsafe {
        ShowWindow(resolved_binding.hwnd, SW_MINIMIZE);
        UpdateWindow(resolved_binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.maximize",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.maximize")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // apply host maximize transition
    unsafe {
        ShowWindow(resolved_binding.hwnd, SW_MAXIMIZE);
        UpdateWindow(resolved_binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Restore one window from minimized or maximized state.
pub(crate) unsafe fn window_restore(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.restore",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.restore")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // apply host restore transition
    unsafe {
        ShowWindow(resolved_binding.hwnd, SW_RESTORE);
        UpdateWindow(resolved_binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
