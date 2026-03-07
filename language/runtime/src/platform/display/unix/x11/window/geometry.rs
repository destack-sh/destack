use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as XprotoConnectionExt};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition,
    WindowSizeConstraints, WindowVisibility, WindowWindowedModeOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::{
    appearance, apply_fullscreen_state, apply_maximized_state, apply_window_size_hints, lifecycle,
    mode_display,
};
use crate::platform::display::unix::x11::{core, event, resource as display_resource};

/// Validate one optional size-constraint payload.
pub(crate) fn validate_size_constraints(
    constraints: Option<WindowSizeConstraints>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(constraints) = constraints else {
        return Ok(());
    };

    // validate minimum constraint values when present
    if let Some(minimum) = constraints.min {
        if !minimum.width.is_finite()
            || !minimum.height.is_finite()
            || minimum.width <= 0.0
            || minimum.height <= 0.0
        {
            return Err(core_platform::invalid_argument(
                "constraints",
                format!("{operation}: minimum logical size must be finite and greater than zero"),
            ));
        }
    }

    // validate maximum constraint values when present
    if let Some(maximum) = constraints.max {
        if !maximum.width.is_finite()
            || !maximum.height.is_finite()
            || maximum.width <= 0.0
            || maximum.height <= 0.0
        {
            return Err(core_platform::invalid_argument(
                "constraints",
                format!("{operation}: maximum logical size must be finite and greater than zero"),
            ));
        }
    }

    // validate min and max ordering when both lanes are present
    if let (Some(minimum), Some(maximum)) = (constraints.min, constraints.max)
        && (minimum.width > maximum.width || minimum.height > maximum.height)
    {
        return Err(core_platform::invalid_argument(
            "constraints",
            format!("{operation}: minimum logical size must not exceed maximum logical size"),
        ));
    }

    Ok(())
}

/// Set one window mode.
pub(crate) unsafe fn window_set_mode(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setMode")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setMode",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = resolved_host_state.clone();

    // validate mode display relation for exclusive fullscreen options
    let display = mode_display(mode);
    if let Some(display) = display {
        display_resource::resolve_display_id(binding, display, "destack.display.window.setMode")?;
    }

    // restore one previous exclusive mode before applying this transition
    let previous_restore = resolved_host_state.exclusive_restore.clone();
    if let Some(restore) = previous_restore {
        lifecycle::restore_exclusive_mode(
            binding,
            &runtime_state,
            &restore,
            "destack.display.window.setMode",
        )?;
        resolved_host_state.exclusive_restore = None;
    }

    // apply requested exclusive mode and roll it back when fullscreen apply fails
    let next_restore = lifecycle::apply_exclusive_mode(
        binding,
        &runtime_state,
        mode,
        "destack.display.window.setMode",
    )?;
    let previous_mode = resolved_host_state.mode;
    let previous_display = resolved_host_state.display;
    let previous_exclusive_restore = resolved_host_state.exclusive_restore.clone();
    resolved_host_state.exclusive_restore = next_restore;
    resolved_host_state.mode = mode;
    resolved_host_state.display = display;
    if let Err(error) =
        apply_fullscreen_state(connection_state.as_ref(), resolved_host_state.window, mode)
    {
        // rollback monitor mode transition when fullscreen state apply fails
        if let Some(restore) = resolved_host_state.exclusive_restore.as_ref() {
            lifecycle::restore_exclusive_mode(
                binding,
                &runtime_state,
                restore,
                "destack.display.window.setMode.rollback",
            )?;
        }

        resolved_host_state.mode = previous_mode;
        resolved_host_state.display = previous_display;
        resolved_host_state.exclusive_restore = previous_exclusive_restore;
        return Err(error);
    }
    let current = resolved_host_state.clone();
    drop(resolved_host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setPosition")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setPosition",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous = resolved_host_state.clone();

    // apply host configure request and publish state delta
    let previous_position = resolved_host_state.position;
    connection_state
        .connection
        .configure_window(
            resolved_host_state.window,
            &ConfigureWindowAux::new().x(position.x).y(position.y),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setPosition",
                format!("configure_window failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setPosition",
            format!("flush failed: {error}"),
        )
    })?;
    resolved_host_state.position = position;
    drop(resolved_host_state);

    event::publish_window_position_changed(
        &runtime_state,
        window_handle,
        previous_position,
        position,
    );

    Ok(())
}

/// Set logical size constraints.
pub(crate) unsafe fn window_set_size_constraints(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // validate optional constraints payload
    validate_size_constraints(constraints, "destack.display.window.setSizeConstraints")?;

    // resolve runtime and mutate the target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizeConstraints")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply one host normal-hints mutation before updating the snapshot
    apply_window_size_hints(
        connection_state.as_ref(),
        resolved_host_state.window,
        resolved_host_state.resizable,
        constraints,
        resolved_host_state.aspect_ratio,
        resolved_host_state.size_physical,
        "destack.display.window.setSizeConstraints",
    )?;
    resolved_host_state.constraints = constraints;

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    size: WindowLogicalSize,
) -> RuntimeResult<()> {
    // validate size payload
    if size.width <= 0.0 || size.height <= 0.0 {
        return Err(core_platform::invalid_argument(
            "size",
            "window logical width and height must be greater than zero",
        ));
    }

    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizeLogical")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setSizeLogical",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply host configure request and publish state delta
    let previous_size_logical = resolved_host_state.size_logical;
    let previous_size_physical = resolved_host_state.size_physical;
    let size_physical = WindowPhysicalSize {
        width: size.width.round().max(1.0) as u32,
        height: size.height.round().max(1.0) as u32,
    };
    connection_state
        .connection
        .configure_window(
            resolved_host_state.window,
            &ConfigureWindowAux::new()
                .width(size_physical.width)
                .height(size_physical.height),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setSizeLogical",
                format!("configure_window failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setSizeLogical",
            format!("flush failed: {error}"),
        )
    })?;
    resolved_host_state.size_logical = size;
    resolved_host_state.size_physical = size_physical;

    // refresh host normal hints for size-locked windows
    apply_window_size_hints(
        connection_state.as_ref(),
        resolved_host_state.window,
        resolved_host_state.resizable,
        resolved_host_state.constraints,
        resolved_host_state.aspect_ratio,
        resolved_host_state.size_physical,
        "destack.display.window.setSizeLogical",
    )?;
    drop(resolved_host_state);

    event::publish_window_size_changed(
        &runtime_state,
        window_handle,
        previous_size_logical,
        previous_size_physical,
        size,
        size_physical,
    );

    Ok(())
}

/// Set one physical window size.
pub(crate) unsafe fn window_set_size_physical(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    size: WindowPhysicalSize,
) -> RuntimeResult<()> {
    // validate size payload
    if size.width == 0 || size.height == 0 {
        return Err(core_platform::invalid_argument(
            "size",
            "window physical width and height must be greater than zero",
        ));
    }

    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizePhysical")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setSizePhysical",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply host configure request and publish state delta
    let previous_size_logical = resolved_host_state.size_logical;
    let previous_size_physical = resolved_host_state.size_physical;
    connection_state
        .connection
        .configure_window(
            resolved_host_state.window,
            &ConfigureWindowAux::new()
                .width(size.width)
                .height(size.height),
        )
        .map_err(|error| {
            core::io_error(
                "destack.display.window.setSizePhysical",
                format!("configure_window failed: {error}"),
            )
        })?;
    connection_state.connection.flush().map_err(|error| {
        core::io_error(
            "destack.display.window.setSizePhysical",
            format!("flush failed: {error}"),
        )
    })?;
    resolved_host_state.size_physical = size;
    resolved_host_state.size_logical = WindowLogicalSize {
        width: size.width as f64,
        height: size.height as f64,
    };

    // refresh host normal hints for size-locked windows
    apply_window_size_hints(
        connection_state.as_ref(),
        resolved_host_state.window,
        resolved_host_state.resizable,
        resolved_host_state.constraints,
        resolved_host_state.aspect_ratio,
        resolved_host_state.size_physical,
        "destack.display.window.setSizePhysical",
    )?;
    let current_size_logical = resolved_host_state.size_logical;
    drop(resolved_host_state);

    event::publish_window_size_changed(
        &runtime_state,
        window_handle,
        previous_size_logical,
        previous_size_physical,
        current_size_logical,
        size,
    );

    Ok(())
}

/// Set one window aspect-ratio lock.
pub(crate) unsafe fn window_set_aspect_ratio(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
    aspect_ratio: Option<WindowAspectRatio>,
) -> RuntimeResult<()> {
    // validate optional aspect-ratio payload
    if let Some(aspect_ratio) = aspect_ratio
        && (aspect_ratio.numerator == 0 || aspect_ratio.denominator == 0)
    {
        return Err(core_platform::invalid_argument(
            "aspectRatio",
            "aspect ratio numerator and denominator must both be greater than zero",
        ));
    }

    // resolve runtime and mutate the target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setAspectRatio")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.setAspectRatio",
    )?;
    let mut resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // apply one host normal-hints mutation before updating the snapshot
    apply_window_size_hints(
        connection_state.as_ref(),
        resolved_host_state.window,
        resolved_host_state.resizable,
        resolved_host_state.constraints,
        aspect_ratio,
        resolved_host_state.size_physical,
        "destack.display.window.setAspectRatio",
    )?;
    resolved_host_state.aspect_ratio = aspect_ratio;
    let current = resolved_host_state.clone();
    drop(resolved_host_state);

    // publish all affected state deltas
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe {
        appearance::window_set_visibility(binding, window_handle, WindowVisibility::Minimized)
    }
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.maximize")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.maximize",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let window_id = resolved_host_state.window;
    drop(resolved_host_state);

    // request maximized state through EWMH
    apply_maximized_state(connection_state.as_ref(), window_id, true)?;

    // update visibility state
    unsafe {
        appearance::window_set_visibility(binding, window_handle, WindowVisibility::Maximized)
    }
}

/// Restore one window.
pub(crate) unsafe fn window_restore(
    binding: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and target window host state
    let runtime_state = core::runtime_state(binding);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.restore")?;
    let resolved_host_state = display_resource::resolve_window_host_state(
        binding,
        window_handle,
        "destack.display.window.restore",
    )?;
    let resolved_host_state = resolved_host_state
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let window_id = resolved_host_state.window;
    drop(resolved_host_state);

    // clear fullscreen and maximized state lanes before mapping visible
    apply_fullscreen_state(
        connection_state.as_ref(),
        window_id,
        WindowModeOptions::WindowWindowedModeOptions(WindowWindowedModeOptions {
            kind: binding.store_string("windowed"),
        }),
    )?;
    apply_maximized_state(connection_state.as_ref(), window_id, false)?;

    // update visibility state
    unsafe { appearance::window_set_visibility(binding, window_handle, WindowVisibility::Visible) }
}
