use super::*;

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // decode and validate title payload
    let title = unsafe { title.as_str()? }.to_string();
    let title_wide = core_platform::wide_from_str("title", &title)?;

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setTitle",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setTitle")?;

    // apply host title update
    let status = unsafe { SetWindowTextW(resolved_binding.hwnd, title_wide.as_ptr()) };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setTitle",
            "SetWindowTextW",
            "failed to set window title",
        ));
    }

    // update cached title value
    resolved_binding.title = title;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setIcons",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setIcons")?;

    // decode icons and build new host icon handles
    let (new_small_icon, new_big_icon) = match icons {
        Some(icon_set) => {
            let decoded = decode_window_icons(icon_set)?;
            let (small_width, small_height, big_width, big_height) =
                icon_target_dimensions(WINDOW_ICON_SMALL_DEFAULT, WINDOW_ICON_BIG_DEFAULT);
            let small_index = best_icon_index(&decoded, small_width, small_height);
            let big_index = best_icon_index(&decoded, big_width, big_height);

            let small_icon =
                create_hicon(&decoded[small_index], "destack.display.window.setIcons")?;
            let big_icon =
                // resolve this variant
                match create_hicon(&decoded[big_index], "destack.display.window.setIcons") {
                    Ok(icon) => icon,
                    Err(error) => {
                        destroy_owned_icons(small_icon, 0);
                        return Err(error);
                    }
                };

            (small_icon, big_icon)
        }
        None => (0, 0),
    };

    // apply host icon handles
    unsafe {
        let _ = SendMessageW(
            resolved_binding.hwnd,
            WM_SETICON,
            ICON_SMALL as usize,
            new_small_icon,
        );
        let _ = SendMessageW(
            resolved_binding.hwnd,
            WM_SETICON,
            ICON_BIG as usize,
            new_big_icon,
        );
    }

    // swap cached icon handles
    let previous_small_icon = resolved_binding.icon_small;
    let previous_big_icon = resolved_binding.icon_big;
    resolved_binding.icon_small = new_small_icon;
    resolved_binding.icon_big = new_big_icon;

    // release stale icon handles
    let stale_small_icon = if previous_small_icon == new_small_icon {
        0
    } else {
        previous_small_icon
    };
    let stale_big_icon = if previous_big_icon == new_big_icon {
        0
    } else {
        previous_big_icon
    };
    destroy_owned_icons(stale_small_icon, stale_big_icon);

    Ok(())
}

/// Set one window visibility state.
pub(crate) unsafe fn window_set_visibility(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setVisibility",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setVisibility")?;

    // capture previous state for delta publication
    let previous = resolved_binding.clone();

    // apply host visibility transition
    unsafe {
        ShowWindow(resolved_binding.hwnd, show_command(visibility));
        UpdateWindow(resolved_binding.hwnd);
    }

    // refresh cached state and publish deltas
    resolved_binding.visibility = visibility;
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window resizable state.
pub(crate) unsafe fn window_set_resizable(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setResizable",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setResizable")?;

    // apply cached mutation and rollback on host failure
    let previous_resizable = resolved_binding.resizable;
    resolved_binding.resizable = resizable;
    // evaluate this condition
    if let Err(error) = apply_window_style(&resolved_binding, "destack.display.window.setResizable")
    {
        resolved_binding.resizable = previous_resizable;
        return Err(error);
    }

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setDecorated",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setDecorated")?;

    // apply cached mutation and rollback on host failure
    let previous_decorated = resolved_binding.decorated;
    resolved_binding.decorated = decorated;
    // evaluate this condition
    if let Err(error) = apply_window_style(&resolved_binding, "destack.display.window.setDecorated")
    {
        resolved_binding.decorated = previous_decorated;
        return Err(error);
    }

    Ok(())
}

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setAlwaysOnTop")?;

    // apply host topmost transition
    let status = unsafe {
        SetWindowPos(
            resolved_binding.hwnd,
            // evaluate this condition
            if alwaysontop {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            },
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE,
        )
    };
    // evaluate this condition
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setAlwaysOnTop",
            "SetWindowPos",
            "failed to update topmost state",
        ));
    }

    // update cached topmost state
    resolved_binding.always_on_top = alwaysontop;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setChrome",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setChrome")?;

    // capture previous state for rollback and delta publication
    let previous = resolved_binding.clone();

    // apply cached mutation and rollback on host failure
    resolved_binding.chrome = chrome;
    // evaluate this condition
    if let Err(error) = apply_window_style(&resolved_binding, "destack.display.window.setChrome") {
        resolved_binding.chrome = previous.chrome;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setMousePassthrough",
    )?;

    // capture previous state for rollback and delta publication
    let previous = resolved_binding.clone();

    // apply cached mutation and rollback on host failure
    resolved_binding.mouse_passthrough = passthrough;
    // evaluate this condition
    if let Err(error) = apply_window_style(
        &resolved_binding,
        "destack.display.window.setMousePassthrough",
    ) {
        resolved_binding.mouse_passthrough = previous.mouse_passthrough;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    // validate opacity payload
    let opacity = normalize_opacity(opacity, "opacity")?;

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setOpacity",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.setOpacity")?;

    // capture previous state for rollback and delta publication
    let previous = resolved_binding.clone();

    // apply cached mutation and rollback on host failure
    resolved_binding.opacity = opacity;
    // evaluate this condition
    if let Err(error) = apply_window_style(&resolved_binding, "destack.display.window.setOpacity") {
        resolved_binding.opacity = previous.opacity;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Read one window opacity value.
pub(crate) unsafe fn window_opacity(
    binding: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.opacity",
    )?;
    let resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&resolved_binding, "destack.display.window.opacity")?;

    // write cached opacity value
    unsafe {
        *out = resolved_binding.opacity;
    }

    Ok(())
}

/// Set one window taskbar-visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    binding: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window resolved_binding
    let resolved_binding = display_resource::resolve_window_binding(
        binding,
        window,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut resolved_binding = resolved_binding
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(
        &resolved_binding,
        "destack.display.window.setTaskbarVisible",
    )?;

    // capture previous state for rollback and delta publication
    let previous = resolved_binding.clone();

    // apply cached mutation and rollback on host failure
    resolved_binding.taskbar_visible = visible;
    // evaluate this condition
    if let Err(error) = apply_window_style(
        &resolved_binding,
        "destack.display.window.setTaskbarVisible",
    ) {
        resolved_binding.taskbar_visible = previous.taskbar_visible;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut resolved_binding);
    let next = resolved_binding.clone();
    drop(resolved_binding);

    let event_runtime_state = event::display_event_runtime_state(binding);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
