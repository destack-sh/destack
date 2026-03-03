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
    context: &BindingCallContext,
    window: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setPosition",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setPosition")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // apply host position update
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            position.x,
            position.y,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOSIZE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setPosition",
            "SetWindowPos",
            "failed to set window position",
        ));
    }

    // refresh cached state and publish deltas
    binding.position = position;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    // validate logical size payload
    let size = normalize_logical_size(size, "size")?;

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizeLogical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizeLogical")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // resolve constrained target size and host outer rectangle
    let locked_size = apply_aspect_ratio_lock(size, binding.aspect_ratio);
    let clamped_size = clamp_logical_size(locked_size, binding.constraints);
    let size_physical = logical_to_physical(clamped_size, binding.scale_factor_milli);
    let style = window_style_for_binding(&binding);
    let ex_style = window_ex_style_for_binding(&binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        binding.hwnd,
        size_physical,
        style,
        ex_style,
        "destack.display.window.setSizeLogical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizeLogical",
            "SetWindowPos",
            "failed to set logical window size",
        ));
    }

    // refresh cached state and publish deltas
    binding.size_logical = clamped_size;
    binding.size_physical = size_physical;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    // validate physical size payload
    let size = normalize_physical_size(size, "size")?;

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizePhysical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizePhysical")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // resolve constrained target size and host outer rectangle
    let size_logical = physical_to_logical(size, binding.scale_factor_milli);
    let size_logical = apply_aspect_ratio_lock(size_logical, binding.aspect_ratio);
    let clamped_logical = clamp_logical_size(size_logical, binding.constraints);
    let clamped_physical = logical_to_physical(clamped_logical, binding.scale_factor_milli);
    let style = window_style_for_binding(&binding);
    let ex_style = window_ex_style_for_binding(&binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        binding.hwnd,
        clamped_physical,
        style,
        ex_style,
        "destack.display.window.setSizePhysical",
    )?;

    // apply host size update
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
            0,
            0,
            0,
            outer_width,
            outer_height,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
    };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setSizePhysical",
            "SetWindowPos",
            "failed to set physical window size",
        ));
    }

    // refresh cached state and publish deltas
    binding.size_logical = clamped_logical;
    binding.size_physical = clamped_physical;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setSizeConstraints")?;

    // update cached constraints
    binding.constraints = constraints;

    Ok(())
}

/// Set one window mode payload.
pub(crate) unsafe fn window_set_mode(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setMode")?;

    // short circuit no-op mode transitions
    if same_window_mode(binding.mode, mode) {
        return Ok(());
    }

    // apply transition and rollback in-memory state on host failure
    let previous = binding.clone();
    if let Err(error) = apply_mode_options(
        context,
        &mut binding,
        mode,
        "destack.display.window.setMode",
        true,
    ) {
        let _ = apply_mode_options(
            context,
            &mut binding,
            previous.mode,
            "destack.display.window.setMode.rollback",
            false,
        );
        binding.mode = previous.mode;
        binding.display = previous.display;
        binding.exclusive_restore = previous.exclusive_restore.clone();
        refresh_window_snapshot(&mut binding);

        return Err(error);
    }

    // refresh cached state and publish semantic deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    if !same_window_mode(previous.mode, next.mode) {
        event::publish_window_mode_event(&event_runtime_state, window, previous.mode, next.mode);
    }
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
    context: &BindingCallContext,
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

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setAspectRatio",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setAspectRatio")?;

    // capture previous state and apply aspect ratio update
    let previous = binding.clone();
    binding.aspect_ratio = aspectratio;

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.minimize",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.minimize")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // apply host minimize transition
    unsafe {
        ShowWindow(binding.hwnd, SW_MINIMIZE);
        UpdateWindow(binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.maximize",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.maximize")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // apply host maximize transition
    unsafe {
        ShowWindow(binding.hwnd, SW_MAXIMIZE);
        UpdateWindow(binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Restore one window from minimized or maximized state.
pub(crate) unsafe fn window_restore(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.restore",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.restore")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // apply host restore transition
    unsafe {
        ShowWindow(binding.hwnd, SW_RESTORE);
        UpdateWindow(binding.hwnd);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
