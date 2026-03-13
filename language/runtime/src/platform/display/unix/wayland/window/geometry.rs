use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition,
    WindowSizeConstraints,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{
    mode_display, mode_display_mode, normalize_logical_size, normalize_physical_size,
    require_xdg_toplevel_id, resolve_window_host_state, same_window_mode,
    validate_size_constraints,
};
use crate::platform::display::unix::wayland::{
    core as wayland_core, event, monitor, resource as display_resource,
};

/// Validate one optional aspect-ratio payload.
fn validate_aspect_ratio(
    aspect_ratio: Option<WindowAspectRatio>,
    field: &'static str,
) -> RuntimeResult<()> {
    let Some(aspect_ratio) = aspect_ratio else {
        return Ok(());
    };

    // reject zero aspect components
    if aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0 {
        return Err(core_platform::invalid_argument(
            field,
            "aspect ratio numerator and denominator must be greater than zero",
        ));
    }

    Ok(())
}

/// Validate one mode display lane and optional display-mode payload.
fn validate_mode_request(
    context: &BindingCallContext,
    mode: WindowModeOptions,
    operation: &'static str,
) -> RuntimeResult<Option<resource::DisplayHandle>> {
    // reject exclusive fullscreen on wayland: this backend only supports compositor fullscreen
    if matches!(
        mode,
        WindowModeOptions::WindowExclusiveFullscreenModeOptions(_)
    ) {
        return Err(core_platform::not_supported(operation));
    }

    // resolve optional display handle and ensure it exists
    let display = mode_display(mode);
    if let Some(display_handle) = display {
        let snapshot = monitor::snapshot_by_display_handle(context, display_handle, operation)?;

        // validate optional exclusive-mode payload against available display modes
        if let Some(display_mode) = mode_display_mode(mode)
            && !snapshot.modes.contains(&display_mode)
        {
            return Err(core_platform::invalid_argument(
                "mode",
                "display mode is not supported by the target display",
            ));
        }
    }

    Ok(display)
}

/// Resolve one optional wayland output object from one display identifier.
fn resolve_mode_target_output(
    dispatch_state: &wayland_core::WaylandConnectionDispatchState,
    display_id: Option<&str>,
    operation: &'static str,
) -> RuntimeResult<Option<wayland_client::protocol::wl_output::WlOutput>> {
    // accept compositor-selected output when request did not provide one
    let Some(display_id) = display_id else {
        return Ok(None);
    };

    // decode one output global name from display id
    let output_global_name = wayland_core::output_global_name_from_display_id(display_id)
        .ok_or_else(|| {
            core_platform::invalid_argument(
                "mode.display",
                format!("{operation}: invalid wayland display id '{display_id}'"),
            )
        })?;

    // resolve one live output object for this global name
    let output = dispatch_state
        .output
        .outputs_by_global
        .get(&output_global_name)
        .cloned()
        .ok_or_else(|| {
            core_platform::io_not_found(
                operation,
                format!("display '{display_id}' is unavailable for mode request"),
            )
        })?;

    Ok(Some(output))
}

/// Apply one mode transition request to one xdg_toplevel.
fn apply_mode_request(
    context: &BindingCallContext,
    toplevel_id: wayland_client::backend::ObjectId,
    mode: WindowModeOptions,
    display: Option<resource::DisplayHandle>,
    operation: &'static str,
) -> RuntimeResult<()> {
    // resolve optional output lane from requested display handle
    let display_id = if let Some(display_handle) = display {
        Some(display_resource::resolve_display_id(
            context,
            display_handle,
            operation,
        )?)
    } else {
        None
    };

    wayland_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, dispatch_state| {
            let toplevel =
                wayland_core::resolve_xdg_toplevel(connection, toplevel_id.clone(), operation)?;

            // map mode requests to xdg-shell fullscreen and maximize requests
            match mode {
                WindowModeOptions::WindowWindowedModeOptions(_) => {
                    toplevel.unset_fullscreen();
                    toplevel.unset_maximized();
                }
                WindowModeOptions::WindowBorderlessModeOptions(_) => {
                    let output = resolve_mode_target_output(
                        dispatch_state,
                        display_id.as_deref(),
                        operation,
                    )?;
                    toplevel.set_fullscreen(output.as_ref());
                }
                WindowModeOptions::WindowExclusiveFullscreenModeOptions(_) => {
                    let output = resolve_mode_target_output(
                        dispatch_state,
                        display_id.as_deref(),
                        operation,
                    )?;
                    toplevel.set_fullscreen(output.as_ref());
                }
            }

            wayland_core::flush_queue(event_queue, operation)?;

            Ok(())
        },
    )
}

/// Apply one resizable and constraint policy to one xdg_toplevel.
fn apply_size_policy(
    context: &BindingCallContext,
    toplevel_id: wayland_client::backend::ObjectId,
    resizable: bool,
    constraints: Option<WindowSizeConstraints>,
    current_size: WindowPhysicalSize,
    operation: &'static str,
) -> RuntimeResult<()> {
    wayland_core::with_connection_dispatch(
        context,
        operation,
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(connection, toplevel_id, operation)?;

            if !resizable {
                let width = current_size.width.min(i32::MAX as u32) as i32;
                let height = current_size.height.min(i32::MAX as u32) as i32;
                toplevel.set_min_size(width.max(1), height.max(1));
                toplevel.set_max_size(width.max(1), height.max(1));
            } else if let Some(constraints) = constraints {
                if let Some(minimum) = constraints.min {
                    let width = minimum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                    let height = minimum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                    toplevel.set_min_size(width, height);
                } else {
                    toplevel.set_min_size(0, 0);
                }

                if let Some(maximum) = constraints.max {
                    let width = maximum.width.round().clamp(1.0, i32::MAX as f64) as i32;
                    let height = maximum.height.round().clamp(1.0, i32::MAX as f64) as i32;
                    toplevel.set_max_size(width, height);
                } else {
                    toplevel.set_max_size(0, 0);
                }
            } else {
                toplevel.set_min_size(0, 0);
                toplevel.set_max_size(0, 0);
            }

            wayland_core::flush_queue(event_queue, operation)?;

            Ok(())
        },
    )
}

/// Set one window mode.
pub(crate) unsafe fn window_set_mode(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // validate mode payload against display topology
    let display = validate_mode_request(context, mode, "destack.display.window.setMode")?;

    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.setMode")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // skip no-op mode transitions
    let previous_mode = host_state.mode;
    if same_window_mode(previous_mode, mode) {
        return Ok(());
    }

    // apply compositor mode request and update runtime snapshot
    let toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.setMode")?;
    apply_mode_request(
        context,
        toplevel_id,
        mode,
        display,
        "destack.display.window.setMode",
    )?;

    host_state.pending_mode = Some(mode);
    drop(host_state);
    wayland_core::dispatch_pending(context, "destack.display.window.setMode")?;

    Ok(())
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn window_set_aspect_ratio(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    aspect_ratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    // validate aspect-ratio payload and resolve window host state
    validate_aspect_ratio(aspect_ratio, "aspectRatio")?;
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setAspectRatio",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // wayland xdg-shell has no standard aspect-ratio request lane
    if aspect_ratio.is_some() {
        return Err(core_platform::not_supported(
            "destack.display.window.setAspectRatio",
        ));
    }

    host_state.aspect_ratio = None;
    let current = host_state.clone();
    drop(host_state);

    // publish all affected state deltas
    let runtime_state = wayland_core::runtime_state(context);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    _position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    resolve_window_host_state(context, window_handle, "destack.display.window.setPosition")?;

    // wayland compositor controls toplevel placement
    Err(core_platform::not_supported(
        "destack.display.window.setPosition",
    ))
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // validate constraints payload and resolve target window host state
    validate_size_constraints(constraints, "constraints")?;
    let host_state = resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // apply size policy and update local snapshot
    let toplevel_id =
        require_xdg_toplevel_id(&host_state, "destack.display.window.setSizeConstraints")?;
    apply_size_policy(
        context,
        toplevel_id,
        host_state.resizable,
        constraints,
        host_state.size_physical,
        "destack.display.window.setSizeConstraints",
    )?;

    host_state.constraints = constraints;

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    // normalize size payload and resolve target window host state
    normalize_logical_size(size, "size")?;
    resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setSizeLogical",
    )?;

    // wayland xdg-shell has no explicit toplevel size request lane
    Err(core_platform::not_supported(
        "destack.display.window.setSizeLogical",
    ))
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    // normalize size payload and resolve target window host state
    normalize_physical_size(size, "size")?;
    resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.setSizePhysical",
    )?;

    // wayland xdg-shell has no explicit toplevel size request lane
    Err(core_platform::not_supported(
        "destack.display.window.setSizePhysical",
    ))
}
