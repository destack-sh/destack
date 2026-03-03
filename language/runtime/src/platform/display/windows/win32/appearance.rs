use super::*;

/// Set one window title string.
pub(crate) unsafe fn window_set_title(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    title: NativeStringRef,
) -> RuntimeResult<()> {
    // decode and validate title payload
    let title = unsafe { title.as_str()? }.to_string();
    let title_wide = core_platform::wide_from_str("title", &title)?;

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setTitle",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setTitle")?;

    // apply host title update
    let status = unsafe { SetWindowTextW(binding.hwnd, title_wide.as_ptr()) };
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setTitle",
            "SetWindowTextW",
            "failed to set window title",
        ));
    }

    // update cached title value
    binding.title = title;

    Ok(())
}

/// Set one window icon set.
pub(crate) unsafe fn window_set_icons(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    icons: Option<WindowIconSet>,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setIcons",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setIcons")?;

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
            binding.hwnd,
            WM_SETICON,
            ICON_SMALL as usize,
            new_small_icon,
        );
        let _ = SendMessageW(binding.hwnd, WM_SETICON, ICON_BIG as usize, new_big_icon);
    }

    // swap cached icon handles
    let previous_small_icon = binding.icon_small;
    let previous_big_icon = binding.icon_big;
    binding.icon_small = new_small_icon;
    binding.icon_big = new_big_icon;

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
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visibility: WindowVisibility,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setVisibility",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setVisibility")?;

    // capture previous state for delta publication
    let previous = binding.clone();

    // apply host visibility transition
    unsafe {
        ShowWindow(binding.hwnd, show_command(visibility));
        UpdateWindow(binding.hwnd);
    }

    // refresh cached state and publish deltas
    binding.visibility = visibility;
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window resizable state.
pub(crate) unsafe fn window_set_resizable(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    resizable: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setResizable",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setResizable")?;

    // apply cached mutation and rollback on host failure
    let previous_resizable = binding.resizable;
    binding.resizable = resizable;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setResizable") {
        binding.resizable = previous_resizable;
        return Err(error);
    }

    Ok(())
}

/// Set window decoration state.
pub(crate) unsafe fn window_set_decorated(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    decorated: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setDecorated",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setDecorated")?;

    // apply cached mutation and rollback on host failure
    let previous_decorated = binding.decorated;
    binding.decorated = decorated;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setDecorated") {
        binding.decorated = previous_decorated;
        return Err(error);
    }

    Ok(())
}

/// Set always-on-top state.
pub(crate) unsafe fn window_set_always_on_top(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    alwaysontop: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setAlwaysOnTop",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setAlwaysOnTop")?;

    // apply host topmost transition
    let status = unsafe {
        SetWindowPos(
            binding.hwnd,
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
    if status == 0 {
        return Err(core::io_error(
            "destack.display.window.setAlwaysOnTop",
            "SetWindowPos",
            "failed to update topmost state",
        ));
    }

    // update cached topmost state
    binding.always_on_top = alwaysontop;

    Ok(())
}

/// Set one window chrome kind.
pub(crate) unsafe fn window_set_chrome(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    chrome: WindowChromeKind,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setChrome",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setChrome")?;

    // capture previous state for rollback and delta publication
    let previous = binding.clone();

    // apply cached mutation and rollback on host failure
    binding.chrome = chrome;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setChrome") {
        binding.chrome = previous.chrome;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window mouse passthrough state.
pub(crate) unsafe fn window_set_mouse_passthrough(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    passthrough: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setMousePassthrough",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setMousePassthrough")?;

    // capture previous state for rollback and delta publication
    let previous = binding.clone();

    // apply cached mutation and rollback on host failure
    binding.mouse_passthrough = passthrough;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setMousePassthrough") {
        binding.mouse_passthrough = previous.mouse_passthrough;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Set one window opacity.
pub(crate) unsafe fn window_set_opacity(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    opacity: f64,
) -> RuntimeResult<()> {
    // validate opacity payload
    let opacity = normalize_opacity(opacity, "opacity")?;

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setOpacity",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setOpacity")?;

    // capture previous state for rollback and delta publication
    let previous = binding.clone();

    // apply cached mutation and rollback on host failure
    binding.opacity = opacity;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setOpacity") {
        binding.opacity = previous.opacity;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}

/// Read one window opacity value.
pub(crate) unsafe fn window_opacity(
    context: &BindingCallContext,
    out: *mut f64,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // validate out pointer
    core_platform::ensure_out(out, "out")?;

    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.opacity",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.opacity")?;

    // write cached opacity value
    unsafe {
        *out = binding.opacity;
    }

    Ok(())
}

/// Set one window taskbar-visibility state.
pub(crate) unsafe fn window_set_taskbar_visible(
    context: &BindingCallContext,
    window: resource::WindowHandle,
    visible: bool,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding = display_resource::resolve_window_binding(
        context,
        window,
        "destack.display.window.setTaskbarVisible",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.setTaskbarVisible")?;

    // capture previous state for rollback and delta publication
    let previous = binding.clone();

    // apply cached mutation and rollback on host failure
    binding.taskbar_visible = visible;
    if let Err(error) = apply_window_style(&binding, "destack.display.window.setTaskbarVisible") {
        binding.taskbar_visible = previous.taskbar_visible;
        return Err(error);
    }

    // refresh cached state and publish deltas
    refresh_window_snapshot(&mut binding);
    let next = binding.clone();
    drop(binding);

    let event_runtime_state = event::display_event_runtime_state(context);
    event::publish_state_deltas(&event_runtime_state, window, &previous, &next);

    Ok(())
}
