use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{HINSTANCE, HWND};
use windows_sys::Win32::Graphics::Gdi::UpdateWindow;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DestroyWindow, GetForegroundWindow, SetForegroundWindow,
    ShowWindow,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowCursorIcon, WindowCursorMode, WindowOptions, WindowPosition, WindowRole, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::core::{
    current_window_theme, normalize_opacity, show_command_on_open, window_ex_style_for_host_state,
    window_style_for_host_state,
};
use super::cursor::{
    refresh_cursor_policy, refresh_cursor_policy_best_effort, remove_cursor_policy,
    upsert_cursor_policy,
};
use super::drop::{register_window_drop_target, unregister_window_drop_target};
use super::geometry::{
    clamp_logical_size, dpi_from_scale_factor_milli, logical_to_physical, normalize_logical_size,
    outer_size_from_client_size_for_dpi, refresh_window_snapshot, window_class_name,
    window_scale_factor_milli,
};
use super::message::ensure_window_class_registered;
use super::mode::{apply_mode_options, restore_exclusive_mode};
use super::options::resolve_role_open_defaults;
use super::relation::{
    apply_modal_owner_transition, apply_owner_relationship, owner_relationship,
    restore_modal_owner_on_close,
};
use crate::platform::display::windows::win32::core::Win32WindowDispatchEntry;
use crate::platform::display::windows::win32::model::Win32WindowHostState;
use crate::platform::display::windows::win32::{
    core, event, monitor, resource as display_resource,
};

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
    let removed = context.worker().resources.remove_and_finalize(
        context.world(),
        handle.0,
        Some(context.engine()),
    );
    if !removed {
        context.warn(
            "display",
            operation,
            "failed to remove runtime window resource during cleanup",
            None,
        );
    }
}

/// Restore any exclusive-mode host side effects after one failed open path.
fn rollback_failed_open_mode(
    context: &BindingCallContext,
    host_state: &mut Win32WindowHostState,
) -> RuntimeResult<()> {
    // skip rollback when no exclusive mode restore state exists
    let Some(restore) = host_state.exclusive_restore.clone() else {
        return Ok(());
    };

    // attempt host rollback and clear cached restore state
    restore_exclusive_mode(
        context,
        &restore,
        "destack.display.window.open.rollback",
        false,
    )?;
    host_state.exclusive_restore = None;

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
    let runtime_state = core::runtime_state(context);
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

    let scale_factor_milli = if let Some(display) = options.display {
        monitor::monitor_snapshot_for_handle(context, display, "destack.display.window.open")?
            .descriptor
            .scale_factor_milli
            .max(1)
    } else {
        monitor::enumerate_monitor_snapshots()?
            .into_iter()
            .find(|snapshot| snapshot.descriptor.primary)
            .map(|snapshot| snapshot.descriptor.scale_factor_milli.max(1))
            .unwrap_or(1000)
    };
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

    if modal && options.parent.is_none() && options.transient_for.is_none() {
        return Err(core_platform::invalid_argument(
            "options.modal",
            "modal windows require parent or transientFor relationship",
        ));
    }

    // build provisional host-state payload used before resource registration
    let initial_mode = options.mode;
    let mut provisional_host_state = Win32WindowHostState {
        id: format!("win32-window-{}", runtime_state.next_window_identifier()),
        hwnd: 0,
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
    let style = window_style_for_host_state(&provisional_host_state);
    let ex_style = window_ex_style_for_host_state(&provisional_host_state);
    let initial_dpi = dpi_from_scale_factor_milli(scale_factor_milli);
    let (outer_width, outer_height) = outer_size_from_client_size_for_dpi(
        provisional_host_state.size_physical,
        style,
        ex_style,
        initial_dpi,
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
            provisional_host_state.position.x,
            provisional_host_state.position.y,
            outer_width,
            outer_height,
            0,
            0,
            GetModuleHandleW(std::ptr::null()) as HINSTANCE,
            std::ptr::null(),
        )
    };
    if hwnd == 0 {
        return Err(core::io_error(
            "destack.display.window.open",
            "CreateWindowExW",
            "failed to create window",
        ));
    }

    // apply initial mode and owner state with rollback on failure
    provisional_host_state.hwnd = hwnd;
    if let Err(error) = apply_mode_options(
        context,
        &mut provisional_host_state,
        initial_mode,
        "destack.display.window.open",
        true,
    ) {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_host_state);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    if let Err(error) = apply_owner_relationship(
        context,
        &provisional_host_state,
        "destack.display.window.open",
    ) {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_host_state);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }
    if let Err(error) = apply_modal_owner_transition(
        context,
        None,
        false,
        owner_relationship(&provisional_host_state),
        provisional_host_state.modal,
        "destack.display.window.open",
    ) {
        let rollback_result = rollback_failed_open_mode(context, &mut provisional_host_state);
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        rollback_result?;
        return Err(error);
    }

    // refresh the host-state snapshot after host setup
    provisional_host_state.scale_factor_milli =
        window_scale_factor_milli(provisional_host_state.hwnd);
    refresh_window_snapshot(&mut provisional_host_state);

    // register runtime resource and hwnd entry
    let host_state = Arc::new(Mutex::new(provisional_host_state));
    let entry = display_resource::window_resource_entry(context, hwnd, Arc::clone(&host_state));
    let resource_id =
        context
            .worker()
            .resources
            .insert(&context.world(), entry, Some(context.engine()));
    let handle = resource::WindowHandle(resource_id);
    {
        let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        upsert_cursor_policy(&runtime_state, handle, &host_state);
    }
    if let Err(error) = refresh_cursor_policy(&runtime_state) {
        remove_cursor_policy(&runtime_state, handle);
        finalize_window_resource_best_effort(
            context,
            handle,
            "destack.display.window.open.cleanup",
        );
        destroy_window_best_effort(context, hwnd, "destack.display.window.open.cleanup");
        return Err(error);
    }

    event::refresh_monitor_topology_cache(context)?;
    if let Err(error) = Win32WindowDispatchEntry::register(
        hwnd,
        Win32WindowDispatchEntry {
            window: handle,
            host_state: Arc::downgrade(&host_state),
            runtime_state: Arc::clone(&runtime_state),
        },
        "destack.display.window.open",
    ) {
        remove_cursor_policy(&runtime_state, handle);
        refresh_cursor_policy_best_effort(&runtime_state);

        let mut snapshot = host_state
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
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        register_window_drop_target(
            &mut host_state,
            handle,
            &runtime_state,
            "destack.display.window.open",
        )
    } {
        remove_cursor_policy(&runtime_state, handle);
        refresh_cursor_policy_best_effort(&runtime_state);

        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        unregister_window_drop_target(&mut host_state);
        let mut snapshot = host_state.clone();
        drop(host_state);

        Win32WindowDispatchEntry::unregister(hwnd);
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
    event::publish_window_created_event(&runtime_state, handle);

    let previous = {
        let host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        Win32WindowHostState {
            visibility: WindowVisibility::Hidden,
            focused: false,
            ..host_state.clone()
        }
    };
    let next = {
        let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
        refresh_window_snapshot(&mut host_state);
        host_state.clone()
    };
    if previous.visibility != next.visibility
        || previous.position != next.position
        || previous.size_logical != next.size_logical
        || previous.size_physical != next.size_physical
        || previous.scale_factor_milli != next.scale_factor_milli
        || previous.focused != next.focused
        || previous.theme != next.theme
    {
        event::publish_state_deltas(&runtime_state, handle, &previous, &next);
    }

    // write opened handle
    unsafe {
        *out = handle;
    }

    Ok(())
}
