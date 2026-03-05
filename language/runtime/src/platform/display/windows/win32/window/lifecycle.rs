use super::drop::{register_window_drop_target, unregister_window_drop_target};
use super::*;

/// Destroy one host window during error unwind and publish diagnostics on failure.
fn destroy_window_best_effort(context: &BindingCallContext, hwnd: HWND, operation: &'static str) {
    // skip destruction when hwnd is no longer valid
    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) } == 0 {
        return;
    }

    // attempt host destruction and publish diagnostics on failure
    let status = unsafe { DestroyWindow(hwnd) };
    if status == 0 {
        context.warn(
            "display",
            operation,
            "failed to destroy window during cleanup",
            Some(core_platform::last_error_code() as u32),
        );
    }
}

/// Finalize one runtime window resource during error unwind and publish diagnostics on failure.
fn finalize_window_resource_best_effort(
    context: &BindingCallContext,
    handle: resource::WindowHandle,
    operation: &'static str,
) {
    // finalize runtime resource and publish diagnostics when resource is missing
    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(handle.0, Some(context.engine()));
    if !removed {
        context.warn(
            "display",
            operation,
            "failed to remove runtime window resource during cleanup",
            None,
        );
    }
}

/// Resolve role-specific open defaults for one window open request.
fn resolve_role_open_defaults(
    role: WindowRole,
    chrome: WindowChromeKind,
    decorated: bool,
    taskbar_visible: bool,
    always_on_top: bool,
) -> (WindowChromeKind, bool, bool, bool) {
    // keep explicit caller values for top-level windows
    if role == WindowRole::Toplevel {
        return (chrome, decorated, taskbar_visible, always_on_top);
    }

    // popup role defaults to popup chrome and hidden taskbar presence
    if role == WindowRole::Popup {
        let resolved_chrome = if chrome == WindowChromeKind::Standard {
            WindowChromeKind::Popup
        } else {
            chrome
        };
        return (resolved_chrome, decorated, false, always_on_top);
    }

    // overlay role defaults to popup chrome, undecorated, topmost, and hidden taskbar presence
    (WindowChromeKind::Popup, false, false, true)
}

/// Restore any exclusive-mode host side effects after one failed open path.
fn rollback_failed_open_mode(
    context: &BindingCallContext,
    binding: &mut Win32WindowBinding,
) -> RuntimeResult<()> {
    // skip rollback when no exclusive mode restore state exists
    let Some(restore) = binding.exclusive_restore.clone() else {
        return Ok(());
    };

    // attempt host rollback and clear cached restore state
    restore_exclusive_mode(
        context,
        &restore,
        "destack.display.window.open.rollback",
        false,
    )?;
    binding.exclusive_restore = None;

    Ok(())
}

/// Open one window instance.
pub(crate) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    // validate output pointer and ensure class registration
    core_platform::ensure_out(out, "out")?;
    ensure_window_class_registered()?;

    // decode and normalize open options
    let title = unsafe { options.title.as_str()? }.to_string();
    let requested_size_logical =
        normalize_logical_size(options.size_logical, "options.sizeLogical")?;
    let (resolved_chrome, resolved_decorated, resolved_taskbar_visible, resolved_always_on_top) =
        resolve_role_open_defaults(
            options.role,
            options.chrome,
            options.decorated,
            options.taskbar_visible,
            options.always_on_top,
        );

    let scale_factor_milli = 1000u32;
    let constrained_logical_size = clamp_logical_size(requested_size_logical, options.constraints);
    let size_physical = logical_to_physical(constrained_logical_size, scale_factor_milli);

    // validate explicit display association when provided
    if let Some(display) = options.display {
        display_resource::resolve_display_id(context, display, "destack.display.window.open")?;
    }

    // validate relationship and opacity constraints
    let opacity = normalize_opacity(options.opacity.unwrap_or(1.0), "options.opacity")?;
    let modal = options.modal.unwrap_or(false);

    // popup windows require one owner relationship
    if options.role == WindowRole::Popup
        && options.parent.is_none()
        && options.transient_for.is_none()
    {
        return Err(core_platform::invalid_argument(
            "options.role",
            "popup windows require parent or transientFor relationship",
        ));
    }

    // evaluate this condition
    if modal && options.parent.is_none() && options.transient_for.is_none() {
        return Err(core_platform::invalid_argument(
            "options.modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // build provisional binding state used before resource registration
    let initial_mode = options.mode;
    let mut provisional_binding = Win32WindowBinding {
        id: format!("win32-window-{}", next_window_identifier(context)),
        hwnd: 0,
        owner_thread_id: current_thread_id(),
        title: title.clone(),
        role: options.role,
        mode: initial_mode,
        display: options.display,
        resizable: options.resizable,
        decorated: resolved_decorated,
        chrome: resolved_chrome,
        taskbar_visible: resolved_taskbar_visible,
        transparent: options.transparent,
        opacity,
        always_on_top: resolved_always_on_top,
        parent: options.parent,
        transient_for: options.transient_for,
        modal,
        mouse_passthrough: options.mouse_passthrough.unwrap_or(false),
        aspect_ratio: options.aspect_ratio,
        visibility: options.visibility,
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        icon_small: 0,
        icon_big: 0,
        position: options.position.unwrap_or(WindowPosition {
            x: CW_USEDEFAULT,
            y: CW_USEDEFAULT,
        }),
        size_logical: constrained_logical_size,
        size_physical,
        scale_factor_milli,
        focused: options.focus_on_show,
        safe_area_insets: None,
        theme: current_window_theme(),
        exclusive_restore: None,
        close_requested_emitted: false,
        destroyed_emitted: false,
        drop_target_callback: 0,
        drop_target_ole_initialized: false,
    };

    // resolve host styles and outer size from requested client size
    let style = window_style_for_binding(&provisional_binding);
    let ex_style = window_ex_style_for_binding(&provisional_binding);
    let (outer_width, outer_height) = outer_size_from_client_size(
        0,
        provisional_binding.size_physical,
        style,
        ex_style,
        "destack.display.window.open",
    )?;

    // create host window instance
    let class_name = window_class_name();
    let title_wide = core_platform::wide_from_str("title", &title)?;
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class_name.as_ptr(),
            title_wide.as_ptr(),
            style,
            provisional_binding.position.x,
            provisional_binding.position.y,
            outer_width,
            outer_height,
            0,
            0,
            GetModuleHandleW(std::ptr::null()) as HINSTANCE,
            std::ptr::null(),
        )
    };
    // evaluate this condition
    if hwnd == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "CreateWindowExW",
            "failed to create window",
        ));
    }

    // apply initial mode and owner state with rollback on failure
    provisional_binding.hwnd = hwnd;
    // evaluate this condition
    if let Err(error) = apply_mode_options(
        context,
        &mut provisional_binding,
        initial_mode,
        "destack.display.window.open",
        true,
    ) {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_binding);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    // evaluate this condition
    if let Err(error) =
        apply_owner_relationship(context, &provisional_binding, "destack.display.window.open")
    {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_binding);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }
    // evaluate this condition
    if let Err(error) = apply_modal_owner_transition(
        context,
        None,
        false,
        owner_relationship(&provisional_binding),
        provisional_binding.modal,
        "destack.display.window.open",
    ) {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_binding);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    // refresh binding snapshot after host setup
    provisional_binding.scale_factor_milli = window_scale_factor_milli(provisional_binding.hwnd);
    refresh_window_snapshot(&mut provisional_binding);

    // register runtime resource and hwnd entry
    let binding = Arc::new(Mutex::new(provisional_binding));
    let entry = display_resource::window_resource_entry(hwnd, Arc::clone(&binding));
    let resource_id = context
        .runtime()
        .resources
        .insert(entry, Some(context.engine()));
    let handle = resource::WindowHandle(resource_id);
    let window_runtime_state = window_runtime_state(context);
    {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        upsert_cursor_policy(&window_runtime_state, handle, &binding);
    }
    // evaluate this condition
    if let Err(error) = refresh_cursor_policy(&window_runtime_state) {
        remove_cursor_policy(&window_runtime_state, handle);
        finalize_window_resource_best_effort(
            context,
            handle,
            "destack.display.window.open.cleanup",
        );
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        return Err(error);
    }

    event::refresh_monitor_topology_cache(context)?;
    let event_runtime_state = event::display_event_runtime_state(context);
    // evaluate this condition
    if let Err(error) = register_runtime_window(
        hwnd,
        WindowRuntimeEntry {
            window: handle,
            binding: Arc::downgrade(&binding),
            event_runtime_state: Arc::clone(&event_runtime_state),
            window_runtime_state: Arc::clone(&window_runtime_state),
        },
        "destack.display.window.open",
    ) {
        remove_cursor_policy(&window_runtime_state, handle);
        refresh_cursor_policy_best_effort(&window_runtime_state);

        let mut snapshot = binding
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let rollback_result = rollback_failed_open_mode(context, &mut snapshot);
        restore_modal_owner_on_close(context, &snapshot);
        finalize_window_resource_best_effort(
            context,
            handle,
            "destack.display.window.open.cleanup",
        );
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    // register native drop target and unwind all state on failure
    if let Err(error) = {
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        register_window_drop_target(
            &mut binding,
            handle,
            &event_runtime_state,
            "destack.display.window.open",
        )
    } {
        remove_cursor_policy(&window_runtime_state, handle);
        refresh_cursor_policy_best_effort(&window_runtime_state);

        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        unregister_window_drop_target(&mut binding);
        let mut snapshot = binding.clone();
        drop(binding);

        unregister_runtime_window(hwnd);
        let rollback_result = rollback_failed_open_mode(context, &mut snapshot);
        restore_modal_owner_on_close(context, &snapshot);
        finalize_window_resource_best_effort(
            context,
            handle,
            "destack.display.window.open.cleanup",
        );
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    // show the window with requested visibility and focus policy
    if options.visibility != WindowVisibility::Hidden {
        unsafe {
            ShowWindow(
                hwnd,
                show_command_on_open(options.visibility, options.focus_on_show),
            );
            UpdateWindow(hwnd);
        }
    }

    // force foreground when explicitly requested
    if options.focus_on_show && options.visibility == WindowVisibility::Visible {
        let status = unsafe { SetForegroundWindow(hwnd) };
        // evaluate this condition
        if status == 0 && unsafe { GetForegroundWindow() } != hwnd {
            context.warn(
                "display",
                "destack.display.window.open",
                "focusOnShow could not grant foreground focus",
                Some(core_platform::last_error_code() as u32),
            );
        }
    }

    // publish created event and state deltas
    event::publish_window_created_event(&event_runtime_state, handle);

    let previous = {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        Win32WindowBinding {
            visibility: WindowVisibility::Hidden,
            focused: false,
            ..binding.clone()
        }
    };
    let next = {
        let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        refresh_window_snapshot(&mut binding);
        binding.clone()
    };
    // evaluate this condition
    if previous.visibility != next.visibility
        || previous.position != next.position
        || previous.size_logical != next.size_logical
        || previous.size_physical != next.size_physical
        || previous.scale_factor_milli != next.scale_factor_milli
        || previous.focused != next.focused
        || previous.theme != next.theme
    {
        event::publish_state_deltas(&event_runtime_state, handle, &previous, &next);
    }

    // write opened handle
    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve and validate the target window binding
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.close")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    ensure_window_thread(&binding, "destack.display.window.close")?;

    // restore exclusive mode if this window owns one
    if let Some(restore) = binding.exclusive_restore.clone() {
        restore_exclusive_mode(context, &restore, "destack.display.window.close", true)?;
        binding.exclusive_restore = None;
    }

    // restore process global side effects and detach drop target
    let runtime_state = window_runtime_state(context);
    restore_cursor_after_close(&runtime_state, window);
    restore_modal_owner_on_close(context, &binding);
    unregister_window_drop_target(&mut binding);

    // clear and detach host icon handles
    let hwnd = binding.hwnd;
    let previous_small_icon = binding.icon_small;
    let previous_big_icon = binding.icon_big;
    binding.icon_small = 0;
    binding.icon_big = 0;

    // compute lifecycle event emission state
    let should_emit_close_requested = !binding.close_requested_emitted;
    // evaluate this condition
    if should_emit_close_requested {
        binding.close_requested_emitted = true;
    }
    drop(binding);
    let event_runtime_state = event::display_event_runtime_state(context);

    // publish close requested when this is the first close path
    if should_emit_close_requested {
        event::publish_window_close_requested_event(&event_runtime_state, window);
    }

    // clear host icons and release owned handles
    unsafe {
        let _ = SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, 0);
        let _ = SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, 0);
    }
    destroy_owned_icons(previous_small_icon, previous_big_icon);

    // destroy host window and pump pending messages
    if unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindow(hwnd) } != 0 {
        let status = unsafe { DestroyWindow(hwnd) };
        // evaluate this condition
        if status == 0 {
            return Err(core::io_error(
                "destack.display.window.close",
                "DestroyWindow",
                "failed to destroy window",
            ));
        }
        pump_window_messages(context)?;
    }

    // emit destroyed once and clear hwnd runtime registration
    let binding =
        display_resource::resolve_window_binding(context, window, "destack.display.window.close")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    // evaluate this condition
    if !binding.destroyed_emitted {
        binding.destroyed_emitted = true;
        drop(binding);
        event::publish_window_destroyed_event(&event_runtime_state, window);
        unregister_runtime_window(hwnd);
    } else {
        drop(binding);
    }

    // remove finalized resource entry
    let removed = context
        .runtime()
        .resources
        .remove_and_finalize(window.0, Some(context.engine()));
    // evaluate this condition
    if !removed {
        return Err(core_platform::io_not_found(
            "destack.display.window.close",
            format!("window handle {} was not found", window.0.0),
        ));
    }

    Ok(())
}
