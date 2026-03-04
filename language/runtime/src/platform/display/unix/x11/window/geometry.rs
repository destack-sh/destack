use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConfigureWindowAux, ConnectionExt as XprotoConnectionExt};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::{
    WindowAspectRatio, WindowLogicalSize, WindowModeOptions, WindowPhysicalSize, WindowPosition,
    WindowSizeConstraints, WindowVisibility, WindowWindowedModeOptions,
};
use crate::platform::{core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::super::{core, event, resource as display_resource};
use super::{appearance, lifecycle};

/// Validate one optional size-constraint payload.
pub(super) fn validate_size_constraints(
    constraints: Option<WindowSizeConstraints>,
    operation: &'static str,
) -> RuntimeResult<()> {
    let Some(constraints) = constraints else {
        return Ok(());
    };

    // validate minimum constraint values when present
    if let Some(minimum) = constraints.min {
        // evaluate this condition
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
        // evaluate this condition
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
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    mode: WindowModeOptions,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setMode")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setMode",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setMode")?;

    // validate mode display relation for exclusive fullscreen options
    let display = super::mode_display(mode);
    // evaluate this condition
    if let Some(display) = display {
        display_resource::resolve_display_id(context, display, "destack.display.window.setMode")?;
    }

    // restore one previous exclusive mode before applying this transition
    let previous_restore = binding.exclusive_restore.clone();
    // evaluate this condition
    if let Some(restore) = previous_restore {
        lifecycle::restore_exclusive_mode(
            context,
            &runtime_state,
            &restore,
            "destack.display.window.setMode",
        )?;
        binding.exclusive_restore = None;
    }

    // apply requested exclusive mode and roll it back when fullscreen apply fails
    let next_restore = lifecycle::apply_exclusive_mode(
        context,
        &runtime_state,
        mode,
        "destack.display.window.setMode",
    )?;
    let previous_mode = binding.mode;
    let previous_display = binding.display;
    let previous_exclusive_restore = binding.exclusive_restore.clone();
    binding.exclusive_restore = next_restore;
    binding.mode = mode;
    binding.display = display;
    // evaluate this condition
    if let Err(error) =
        super::apply_fullscreen_state(connection_state.as_ref(), binding.window, mode)
    {
        // rollback monitor mode transition when fullscreen state apply fails
        if let Some(restore) = binding.exclusive_restore.as_ref() {
            lifecycle::restore_exclusive_mode(
                context,
                &runtime_state,
                restore,
                "destack.display.window.setMode.rollback",
            )?;
        }

        binding.mode = previous_mode;
        binding.display = previous_display;
        binding.exclusive_restore = previous_exclusive_restore;
        return Err(error);
    }
    drop(binding);

    // publish mode-changed event
    event::publish_window_mode_changed(&runtime_state, window_handle, previous_mode, mode);

    Ok(())
}

/// Set one window position.
pub(crate) unsafe fn window_set_position(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    position: WindowPosition,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setPosition")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setPosition",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setPosition")?;

    // apply host configure request and publish state delta
    let previous_position = binding.position;
    connection_state
        .connection
        .configure_window(
            binding.window,
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
    binding.position = position;
    drop(binding);

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
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
    constraints: Option<WindowSizeConstraints>,
) -> RuntimeResult<()> {
    // validate optional constraints payload
    validate_size_constraints(constraints, "destack.display.window.setSizeConstraints")?;

    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizeConstraints")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setSizeConstraints",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setSizeConstraints")?;

    // apply one host normal-hints mutation before updating the snapshot
    super::apply_window_size_hints(
        connection_state.as_ref(),
        binding.window,
        binding.resizable,
        constraints,
        binding.aspect_ratio,
        binding.size_physical,
        "destack.display.window.setSizeConstraints",
    )?;
    binding.constraints = constraints;

    Ok(())
}

/// Set one logical window size.
pub(crate) unsafe fn window_set_size_logical(
    context: &BindingCallContext,
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

    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizeLogical")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setSizeLogical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setSizeLogical")?;

    // apply host configure request and publish state delta
    let previous_size_logical = binding.size_logical;
    let previous_size_physical = binding.size_physical;
    let size_physical = WindowPhysicalSize {
        width: size.width.round().max(1.0) as u32,
        height: size.height.round().max(1.0) as u32,
    };
    connection_state
        .connection
        .configure_window(
            binding.window,
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
    binding.size_logical = size;
    binding.size_physical = size_physical;

    // refresh host normal hints for size-locked windows
    super::apply_window_size_hints(
        connection_state.as_ref(),
        binding.window,
        binding.resizable,
        binding.constraints,
        binding.aspect_ratio,
        binding.size_physical,
        "destack.display.window.setSizeLogical",
    )?;
    drop(binding);

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
    context: &BindingCallContext,
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

    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setSizePhysical")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setSizePhysical",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setSizePhysical")?;

    // apply host configure request and publish state delta
    let previous_size_logical = binding.size_logical;
    let previous_size_physical = binding.size_physical;
    connection_state
        .connection
        .configure_window(
            binding.window,
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
    binding.size_physical = size;
    binding.size_logical = WindowLogicalSize {
        width: size.width as f64,
        height: size.height as f64,
    };

    // refresh host normal hints for size-locked windows
    super::apply_window_size_hints(
        connection_state.as_ref(),
        binding.window,
        binding.resizable,
        binding.constraints,
        binding.aspect_ratio,
        binding.size_physical,
        "destack.display.window.setSizePhysical",
    )?;
    let current_size_logical = binding.size_logical;
    drop(binding);

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
    context: &BindingCallContext,
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

    // resolve runtime and mutate binding state
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.setAspectRatio")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.setAspectRatio",
    )?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.setAspectRatio")?;

    // apply one host normal-hints mutation before updating the snapshot
    super::apply_window_size_hints(
        connection_state.as_ref(),
        binding.window,
        binding.resizable,
        binding.constraints,
        aspect_ratio,
        binding.size_physical,
        "destack.display.window.setAspectRatio",
    )?;
    binding.aspect_ratio = aspect_ratio;

    Ok(())
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    unsafe {
        appearance::window_set_visibility(context, window_handle, WindowVisibility::Minimized)
    }
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.maximize")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.maximize",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.maximize")?;
    let window_id = binding.window;
    drop(binding);

    // request maximized state through EWMH
    super::apply_maximized_state(connection_state.as_ref(), window_id, true)?;

    // update visibility state
    unsafe {
        appearance::window_set_visibility(context, window_handle, WindowVisibility::Maximized)
    }
}

/// Restore one window.
pub(crate) unsafe fn window_restore(
    context: &BindingCallContext,
    window_handle: resource::WindowHandle,
) -> RuntimeResult<()> {
    // resolve runtime and binding lanes
    let runtime_state = core::runtime_state(context);
    let connection_state =
        core::connection_state(&runtime_state, "destack.display.window.restore")?;
    let binding = display_resource::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.restore",
    )?;
    let binding = binding.lock().unwrap_or_else(|error| error.into_inner());
    super::ensure_window_thread(&binding, "destack.display.window.restore")?;
    let window_id = binding.window;
    drop(binding);

    // clear fullscreen and maximized state lanes before mapping visible
    super::apply_fullscreen_state(
        connection_state.as_ref(),
        window_id,
        WindowModeOptions::WindowWindowedModeOptions(WindowWindowedModeOptions {
            kind: context.store_string("windowed"),
        }),
    )?;
    super::apply_maximized_state(connection_state.as_ref(), window_id, false)?;

    // update visibility state
    unsafe { appearance::window_set_visibility(context, window_handle, WindowVisibility::Visible) }
}
