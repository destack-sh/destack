use std::sync::{Arc, Mutex};

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ConnectionExt as XprotoConnectionExt, CreateWindowAux, EventMask, InputFocus,
    PropMode, WindowClass,
};
use x11rb::wrapper::ConnectionExt as X11WrapperConnectionExt;

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowCursorIcon, WindowCursorMode, WindowLogicalSize, WindowModeOptions, WindowOcclusionState,
    WindowOptions, WindowPhysicalSize, WindowPosition, WindowRole, WindowTheme, WindowVisibility,
    WindowWindowedModeOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{
    apply_exclusive_mode, apply_fullscreen_state, apply_maximized_state, apply_window_chrome,
    apply_window_decorated, apply_window_mouse_passthrough, apply_window_size_hints,
    apply_window_transient_owner, geometry, mode_display, request_window_minimize,
    resolve_role_open_defaults, set_net_wm_state, set_window_title,
};
use crate::platform::display::unix::x11::model::X11WindowHostState;
use crate::platform::display::unix::x11::{core, event, monitor, resource as display_resource};

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
    if options.size_logical.width <= 0.0 || options.size_logical.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            "sizeLogical",
            "window logical width and height must be greater than zero",
        ));
    }
    let opacity = options.opacity.unwrap_or(1.0);
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

    if options.modal == Some(true) && options.transient_for.is_none() && options.parent.is_none() {
        return Err(core_platform::invalid_argument(
            "modal",
            "modal windows require transientFor or parent to be set",
        ));
    }
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
    let mode_display = mode_display(options.mode);
    let preferred_display = mode_display.or(options.display);
    if let Some(display) = preferred_display {
        display_resource::resolve_display_id(context, display, "destack.display.window.open")?;
    }

    // resolve optional transient relationship to one x11 window id
    let transient_for_window = if let Some(transient_handle) = options.transient_for {
        let transient_host_state = display_resource::resolve_window_host_state(
            context,
            transient_handle,
            "destack.display.window.open",
        )?;
        let transient_host_state = transient_host_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Some(transient_host_state.window)
    } else if let Some(parent_handle) = options.parent {
        let parent_host_state = display_resource::resolve_window_host_state(
            context,
            parent_handle,
            "destack.display.window.open",
        )?;
        let parent_host_state = parent_host_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        Some(parent_host_state.window)
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
        | EventMask::KEY_PRESS
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
    set_window_title(connection_state.as_ref(), window, title)?;
    apply_window_transient_owner(
        connection_state.as_ref(),
        window,
        transient_for_window,
        "destack.display.window.open",
    )?;
    apply_window_decorated(
        connection_state.as_ref(),
        window,
        resolved_decorated,
        "destack.display.window.open",
    )?;
    apply_window_chrome(
        connection_state.as_ref(),
        window,
        resolved_chrome,
        "destack.display.window.open",
    )?;
    apply_window_size_hints(
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
    apply_window_mouse_passthrough(
        connection_state.as_ref(),
        window,
        options.mouse_passthrough.unwrap_or(false),
        "destack.display.window.open",
    )?;
    if !resolved_taskbar_visible {
        set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_skip_taskbar,
            true,
        )?;
    }
    if resolved_always_on_top {
        set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_above,
            true,
        )?;
    }
    if options.modal.unwrap_or(false) {
        set_net_wm_state(
            connection_state.as_ref(),
            window,
            connection_state.atoms.net_wm_state_modal,
            true,
        )?;
    }
    apply_fullscreen_state(connection_state.as_ref(), window, options.mode)?;
    if options.visibility == WindowVisibility::Maximized {
        apply_maximized_state(connection_state.as_ref(), window, true)?;
    }
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
        request_window_minimize(
            connection_state.as_ref(),
            window,
            "destack.display.window.open",
        )?;
    }

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
            if let Ok(cookie) = connection.destroy_window(window)
                && let Err(cleanup_error) = cookie.check()
            {
                context.warn(
                    "display",
                    "destack.display.window.open",
                    format!(
                        "destroy_window cleanup failed after mode apply error: {cleanup_error}"
                    ),
                    None,
                );
            }
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

    // build runtime host-state payload for this window
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

    // resolve one stable content scale for this window
    let scale_factor_milli = if let Some(display) = preferred_display {
        monitor::monitor_snapshot_for_handle(context, display, "destack.display.window.open")?
            .descriptor
            .scale_factor_milli
            .max(1)
    } else {
        let snapshots = monitor::enumerate_monitor_snapshots(context)?;

        if snapshots.len() == 1 {
            snapshots[0].descriptor.scale_factor_milli.max(1)
        } else if let Some(snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.descriptor.primary)
        {
            snapshot.descriptor.scale_factor_milli.max(1)
        } else {
            monitor::global_scale_factor_milli(
                connection_state.as_ref(),
                "destack.display.window.open",
            )?
            .unwrap_or(1000)
        }
    };

    let host_state = Arc::new(Mutex::new(X11WindowHostState {
        // x11 fullscreen requests are wm mediated: start from honest current state
        id: format!("{}-window-{window}", core::selected_backend_name()),
        window,
        cursor_handle: None,
        title: title.to_string(),
        role: options.role,
        mode: if matches!(
            options.mode,
            WindowModeOptions::WindowBorderlessModeOptions(_)
                | WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
        ) {
            WindowModeOptions::WindowWindowedModeOptions(WindowWindowedModeOptions {
                kind: match options.mode {
                    WindowModeOptions::WindowBorderlessModeOptions(value) => value.kind,
                    WindowModeOptions::WindowExclusiveFullscreenModeOptions(value) => value.kind,
                    WindowModeOptions::WindowWindowedModeOptions(value) => value.kind,
                },
            })
        } else {
            options.mode
        },
        pending_mode: if matches!(
            options.mode,
            WindowModeOptions::WindowBorderlessModeOptions(_)
                | WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
        ) {
            Some(options.mode)
        } else {
            None
        },
        exclusive_restore,
        display: if matches!(
            options.mode,
            WindowModeOptions::WindowBorderlessModeOptions(_)
                | WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
        ) {
            None
        } else {
            preferred_display
        },
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
        visibility: WindowVisibility::Hidden,
        constraints: options.constraints,
        cursor_visible: true,
        cursor_mode: WindowCursorMode::Normal,
        cursor_icon: WindowCursorIcon::Default,
        position,
        size_logical,
        size_physical,
        scale_factor_milli,
        focused: false,
        occlusion: WindowOcclusionState::Occluded,
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
    let resource_id = context.worker().resources.insert(
        context.world(),
        display_resource::window_resource_entry(
            Arc::clone(&connection_state),
            Arc::clone(&host_state),
        ),
        Some(context.engine()),
    );
    let handle = resource::WindowHandle(resource_id);
    runtime_state.register_xid(window, handle, Arc::downgrade(&host_state));
    event::publish_window_created(&runtime_state, handle);
    runtime_state.service_ingress("destack.display.window.open")?;

    unsafe {
        *out = handle;
    }

    Ok(())
}
