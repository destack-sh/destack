use std::sync::{Arc, Mutex};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as XprotoConnectionExt, CreateWindowAux, EventMask, InputFocus,
    PropMode, WindowClass,
};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowChromeKind, WindowCursorIcon, WindowCursorMode, WindowLogicalSize, WindowModeOptions,
    WindowOptions, WindowPhysicalSize, WindowPosition, WindowRole, WindowTheme, WindowVisibility,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::model::{ExclusiveModeRestore, X11WindowBinding};
use super::super::super::{core, event, monitor, resource as display_resource};
use super::{cursor, geometry};

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

/// Apply one exclusive fullscreen monitor mode request and return restore metadata.
pub(super) fn apply_exclusive_mode(
    context: &BindingCallContext,
    runtime_state: &Arc<core::X11RuntimeState>,
    mode: WindowModeOptions,
    operation: &'static str,
) -> RuntimeResult<Option<ExclusiveModeRestore>> {
    // skip monitor mode mutation when this mode is not exclusive fullscreen
    let Some(display) = super::mode_display(mode) else {
        return Ok(None);
    };
    let Some(requested_mode) = super::mode_display_mode(mode) else {
        return Ok(None);
    };

    // resolve one stable display id and current monitor mode
    let display_id = display_resource::resolve_display_id(context, display, operation)?;
    let Some(snapshot) = monitor::monitor_snapshot_by_display_id(context, &display_id)? else {
        return Err(core_platform::io_not_found(
            operation,
            format!("display id '{display_id}' is no longer available"),
        ));
    };
    let previous_mode = snapshot.current_mode;
    // evaluate this condition
    if previous_mode == requested_mode {
        return Ok(None);
    }

    // apply one mode transition and publish monitor mode change
    let current_mode =
        monitor::apply_monitor_mode_by_display_id(context, &display_id, requested_mode, operation)?;
    let current_snapshot = monitor::monitor_snapshot_by_display_id(context, &display_id)?;
    event::publish_monitor_mode_changed(
        runtime_state,
        &display_id,
        Some(previous_mode),
        current_mode,
    );
    // evaluate this condition
    if let Some(current_snapshot) = current_snapshot
        && current_snapshot.descriptor != snapshot.descriptor
    {
        event::publish_monitor_descriptor_changed(
            runtime_state,
            Some(snapshot.descriptor),
            current_snapshot.descriptor,
        );
    }
    event::refresh_monitor_topology_cache(context)?;

    Ok(Some(ExclusiveModeRestore {
        display_id,
        previous_mode,
    }))
}

/// Restore one monitor mode captured by one exclusive fullscreen transition.
pub(super) fn restore_exclusive_mode(
    context: &BindingCallContext,
    runtime_state: &Arc<core::X11RuntimeState>,
    restore: &ExclusiveModeRestore,
    operation: &'static str,
) -> RuntimeResult<()> {
    // capture previous mode when display is still available
    let previous_snapshot = monitor::monitor_snapshot_by_display_id(context, &restore.display_id)?;
    let previous_mode = previous_snapshot
        .as_ref()
        .map(|snapshot| snapshot.current_mode);
    let previous_descriptor = previous_snapshot.map(|snapshot| snapshot.descriptor);

    // apply one restore transition and publish monitor mode delta
    let current_mode = monitor::apply_monitor_mode_by_display_id(
        context,
        &restore.display_id,
        restore.previous_mode,
        operation,
    )?;
    let current_snapshot = monitor::monitor_snapshot_by_display_id(context, &restore.display_id)?;
    // evaluate this condition
    if previous_mode == Some(current_mode) {
        return Ok(());
    }
    event::publish_monitor_mode_changed(
        runtime_state,
        &restore.display_id,
        previous_mode,
        current_mode,
    );
    // evaluate this condition
    if let (Some(previous_descriptor), Some(current_snapshot)) =
        (previous_descriptor, current_snapshot)
        && current_snapshot.descriptor != previous_descriptor
    {
        event::publish_monitor_descriptor_changed(
            runtime_state,
            Some(previous_descriptor),
            current_snapshot.descriptor,
        );
    }
    event::refresh_monitor_topology_cache(context)?;

    Ok(())
}

/// Create one x11 window and register it in runtime resources.
pub(crate) unsafe fn window_open(
    context: &BindingCallContext,
    out: *mut resource::WindowHandle,
    options: WindowOptions,
) -> RuntimeResult<()> {
    // validate out pointer and logical size
    core_platform::ensure_out(out, "out")?;
    let (resolved_chrome, resolved_decorated, resolved_taskbar_visible, resolved_always_on_top) =
        resolve_role_open_defaults(
            options.role,
            options.chrome,
            options.decorated,
            options.taskbar_visible,
            options.always_on_top,
        );

    let title = unsafe { options.title.as_str()? };
    // evaluate this condition
    if options.size_logical.width <= 0.0 || options.size_logical.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            "sizeLogical",
            "window logical width and height must be greater than zero",
        ));
    }
    let opacity = options.opacity.unwrap_or(1.0);
    // evaluate this condition
    if !(0.0..=1.0).contains(&opacity) {
        return Err(core_platform::invalid_argument(
            "opacity",
            "window opacity must be between 0.0 and 1.0",
        ));
    }

    // popup windows require one owner relationship
    if options.role == WindowRole::Popup
        && options.transient_for.is_none()
        && options.parent.is_none()
    {
        return Err(core_platform::invalid_argument(
            "role",
            "popup windows require transientFor or parent to be set",
        ));
    }

    // evaluate this condition
    if options.modal == Some(true) && options.transient_for.is_none() && options.parent.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require transientFor or parent to be set",
        ));
    }
    // evaluate this condition
    if let Some(aspect_ratio) = options.aspect_ratio
        && (aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must both be greater than zero",
        ));
    }
    geometry::validate_size_constraints(options.constraints, "destack.display.window.open")?;

    // resolve runtime and connection state
    let runtime_state = core::runtime_state(context);
    let connection_state = core::connection_state(&runtime_state, "destack.display.window.open")?;
    let connection = &connection_state.connection;
    let screen = connection
        .setup()
        .roots
        .get(connection_state.screen_index)
        .ok_or_else(|| core_platform::invalid_state("x11 setup missing selected screen root"))?;

    // validate requested display preference
    let mode_display = super::mode_display(options.mode);
    let preferred_display = mode_display.or(options.display);
    // evaluate this condition
    if let Some(display) = preferred_display {
        display_resource::resolve_display_id(context, display, "destack.display.window.open")?;
    }

    // resolve optional transient relationship to one x11 window id
    let transient_for_window = if let Some(transient_handle) = options.transient_for {
        let transient_binding = display_resource::resolve_window_binding(
            context,
            transient_handle,
            "destack.display.window.open",
        )?;
        let transient_binding = transient_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        super::ensure_window_thread(&transient_binding, "destack.display.window.open")?;
        Some(transient_binding.window)
    } else if let Some(parent_handle) = options.parent {
        let parent_binding = display_resource::resolve_window_binding(
            context,
            parent_handle,
            "destack.display.window.open",
        )?;
        let parent_binding = parent_binding
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        super::ensure_window_thread(&parent_binding, "destack.display.window.open")?;
        Some(parent_binding.window)
    } else {
        None
    };

    // allocate one x11 window id and host create request
    let window = connection.generate_id().map_err(|error| {
        core::io_error(
            "destack.display.window.open",
            format!("generate_id failed: {error}"),
        )
    })?;
    let x = options.position.map_or(0, |value| value.x) as i16;
    let y = options.position.map_or(0, |value| value.y) as i16;
    let width = options.size_logical.width.max(1.0).round() as u16;
    let height = options.size_logical.height.max(1.0).round() as u16;
    let event_mask = EventMask::EXPOSURE
        | EventMask::FOCUS_CHANGE
        | EventMask::STRUCTURE_NOTIFY
        | EventMask::PROPERTY_CHANGE
        | EventMask::VISIBILITY_CHANGE;
    connection
        .create_window(
            screen.root_depth,
            window,
            screen.root,
            x,
            y,
            width,
            height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .background_pixel(screen.black_pixel)
                .border_pixel(0)
                .event_mask(event_mask),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.open",
                format!("create_window failed: {error}"),
            )
        })?;

    // register WM_DELETE_WINDOW protocol and initial title payload
    connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.wm_protocols,
            AtomEnum::ATOM,
            &[connection_state.atoms.wm_delete_window],
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.open",
                format!("change_property32 failed: {error}"),
            )
        })?;
    connection
        .change_property32(
            PropMode::REPLACE,
            window,
            connection_state.atoms.xdnd_aware,
            AtomEnum::ATOM,
            &[5],
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.open",
                format!("change_property32 failed: {error}"),
            )
        })?;
    super::set_window_title(connection_state.as_ref(), window, title)?;
    super::apply_window_transient_owner(
        connection_state.as_ref(),
        window,
        transient_for_window,
        "destack.display.window.open",
    )?;
    super::apply_window_decorated(
        connection_state.as_ref(),
        window,
        resolved_decorated,
        "destack.display.window.open",
    )?;
    super::apply_window_chrome(
        connection_state.as_ref(),
        window,
        resolved_chrome,
        "destack.display.window.open",
    )?;
    super::apply_window_size_hints(
        connection_state.as_ref(),
        window,
        options.resizable,
        options.constraints,
        options.aspect_ratio,
        WindowPhysicalSize {
            width: width as u32,
            height: height as u32,
        },
        "destack.display.window.open",
    )?;
    super::apply_window_mouse_passthrough(
        connection_state.as_ref(),
        window,
        options.mouse_passthrough.unwrap_or(false),
        "destack.display.window.open",
    )?;
    // evaluate this condition
    if !resolved_taskbar_visible {
        super::set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_skip_taskbar,
            true,
        )?;
    }
    // evaluate this condition
    if resolved_always_on_top {
        super::set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_above,
            true,
        )?;
    }
    // evaluate this condition
    if options.modal.unwrap_or(false) {
        super::set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }
    super::apply_fullscreen_state(connection_state.as_ref(), window, options.mode)?;
    // evaluate this condition
    if options.visibility == WindowVisibility::Maximized {
        super::apply_maximized_state(connection_state.as_ref(), window, true)?;
    }
    // evaluate this condition
    if options.opacity.is_some() {
        let encoded_opacity = (opacity.clamp(0.0, 1.0) * (u32::MAX as f64)).round() as u32;
        connection
            .change_property32(
                PropMode::REPLACE,
                window,
                connection_state.atoms.net_wm_window_opacity,
                AtomEnum::CARDINAL,
                &[encoded_opacity],
            )
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.open",
                    format!("change_property32 failed: {error}"),
                )
            })?;
    }

    // map all non-hidden windows so wm-managed minimize can be requested
    if options.visibility != WindowVisibility::Hidden {
        connection.map_window(window).map_err(|error| {
            core::io_error(
                "destack.display.window.open",
                format!("map_window failed: {error}"),
            )
        })?;
    }

    // request initial minimize through one wm change-state client message
    if options.visibility == WindowVisibility::Minimized {
        super::request_window_minimize(
            connection_state.as_ref(),
            window,
            "destack.display.window.open",
        )?;
    }

    // evaluate this condition
    if options.focus_on_show
        && options.visibility != WindowVisibility::Hidden
        && options.visibility != WindowVisibility::Minimized
    {
        connection
            .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.open",
                    format!("set_input_focus failed: {error}"),
                )
            })?;
    }
    connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.open",
            format!("flush failed: {error}"),
        )
    })?;
    let exclusive_restore = match apply_exclusive_mode(
        context,
        &runtime_state,
        options.mode,
        "destack.display.window.open",
    ) {
        Ok(value) => value,
        Err(error) => {
            // evaluate this condition
            if let Ok(cookie) = connection.destroy_window(window) {
                // evaluate this condition
                if let Err(cleanup_error) = cookie.check() {
                    context.warn(
                        "display",
                        "destack.display.window.open",
                        format!(
                            "destroy_window cleanup failed after mode apply error: {cleanup_error}"
                        ),
                        None,
                    );
                }
            }
            // evaluate this condition
            if let Err(cleanup_error) = connection.flush() {
                context.warn(
                    "display",
                    "destack.display.window.open",
                    format!("flush cleanup failed after mode apply error: {cleanup_error}"),
                    None,
                );
            }
            return Err(error);
        }
    };

    // build runtime binding payload for this window
    let size_logical = WindowLogicalSize {
        width: width as f64,
        height: height as f64,
    };
    let size_physical = WindowPhysicalSize {
        width: width as u32,
        height: height as u32,
    };
    let position = WindowPosition {
        x: i32::from(x),
        y: i32::from(y),
    };
    let binding = Arc::new(Mutex::new(X11WindowBinding {
        id: format!("{}-window-{window}", core::selected_backend_name()),
        window,
        cursor_handle: None,
        owner_thread_id: std::thread::current().id(),
        title: title.to_string(),
        role: options.role,
        mode: options.mode,
        exclusive_restore,
        display: preferred_display,
        resizable: options.resizable,
        decorated: resolved_decorated,
        chrome: resolved_chrome,
        taskbar_visible: resolved_taskbar_visible,
        transparent: options.transparent,
        opacity,
        always_on_top: resolved_always_on_top,
        parent: options.parent,
        transient_for: options.transient_for,
        modal: options.modal.unwrap_or(false),
        mouse_passthrough: options.mouse_passthrough.unwrap_or(false),
        aspect_ratio: options.aspect_ratio,
        visibility: options.visibility,
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        position,
        size_logical,
        size_physical,
        scale_factor_milli: 1000,
        focused: options.focus_on_show
            && options.visibility != WindowVisibility::Hidden
            && options.visibility != WindowVisibility::Minimized,
        safe_area_insets: None,
        theme: WindowTheme::Unknown,
        close_requested_emitted: false,
        destroyed_emitted: false,
        xdnd_source_window: None,
        xdnd_version: None,
        xdnd_types: Vec::new(),
        xdnd_target_type: None,
        xdnd_position: None,
        xdnd_payload: None,
        xdnd_dragging: false,
        xdnd_last_hovered_path: None,
    }));

    // insert resource entry and publish created event
    let resource_id = context.runtime().resources.insert(
        display_resource::window_resource_entry(
            Arc::clone(&connection_state),
            Arc::clone(&binding),
        ),
        Some(context.engine()),
    );
    let handle = resource::WindowHandle(resource_id);
    event::register_xid(&runtime_state, window, handle);
    event::publish_window_created(&runtime_state, handle);

    unsafe {
        *out = handle;
    }

    Ok(())
}

/// Close one window.
pub(crate) unsafe fn window_close(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.close",
    )?;
    let mut binding_snapshot = {
        let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
        super::ensure_window_thread(&binding, "destack.display.window.close")?;
        binding.clone()
    };

    // request host window destruction and flush
    let connection_state = core::connection_state(&runtime_state, "destack.display.window.close")?;
    cursor::release_window_cursor(
        connection_state.as_ref(),
        &mut binding_snapshot,
        "destack.display.window.close",
    )?;
    // evaluate this condition
    if let Some(restore) = binding_snapshot.exclusive_restore.clone() {
        restore_exclusive_mode(
            context,
            &runtime_state,
            &restore,
            "destack.display.window.close",
        )?;
        binding_snapshot.exclusive_restore = None;
    }
    // evaluate this condition
    if !binding_snapshot.destroyed_emitted {
        let cookie = connection_state
            .connection
            .destroy_window(binding_snapshot.window)
            .map_err(|error| {
                core::io_error(
                    "destack.display.window.close",
                    format!("destroy_window failed: {error}"),
                )
            })?;
        cookie.check().map_err(|error| {
            core::io_error(
                "destack.display.window.close",
                format!("destroy_window check failed: {error}"),
            )
        })?;
        connection_state.connection.flush().map_err(|error| {
            core::io_error(
                "destack.display.window.close",
                format!("flush failed: {error}"),
            )
        })?;
    }

    // remove mapping and resource entry
    event::unregister_xid(&runtime_state, binding_snapshot.window);
    let removed = context
        .runtime()
        .resources
        .remove(window_handle.0, Some(context.engine()))
        .is_some();
    // evaluate this condition
    if !removed {
        return Err(core::window_not_found(
            "destack.display.window.close",
            window_handle,
        ));
    }

    // publish destroyed event once for this window
    if !binding_snapshot.destroyed_emitted {
        event::publish_window_destroyed(&runtime_state, window_handle);
    }

    Ok(())
}
