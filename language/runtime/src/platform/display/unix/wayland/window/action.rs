use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowAttentionLevel, WindowResizeEdge, WindowVisibility};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;
use wayland_protocols::xdg::shell::client::xdg_toplevel;

use super::super::{core as backend_core, event};

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    _level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    // resolve target surface from one owner-thread-validated window binding
    let surface_id = super::with_window_binding(
        context,
        window_handle,
        "destack.display.window.requestAttention",
        |binding| Ok(binding.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    backend_core::request_surface_activation(
        context,
        surface_id,
        "destack.display.window.requestAttention",
    )
}

/// Request one redraw for one window.
pub(crate) unsafe fn window_request_refresh(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // validate one live owner-thread window binding
    super::resolve_window_binding(
        context,
        window_handle,
        "destack.display.window.requestRefresh",
    )?;

    // publish refresh-requested event
    let runtime_state = backend_core::runtime_state(context);
    event::publish_window_refresh_requested(&runtime_state, window_handle);

    Ok(())
}

/// Focus one window.
pub(crate) unsafe fn window_focus(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target surface from one owner-thread-validated window binding
    let surface_id = super::with_window_binding(
        context,
        window_handle,
        "destack.display.window.focus",
        |binding| Ok(binding.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    backend_core::request_surface_activation(context, surface_id, "destack.display.window.focus")
}

/// Raise one window.
pub(crate) unsafe fn window_raise(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target surface from one owner-thread-validated window binding
    let surface_id = super::with_window_binding(
        context,
        window_handle,
        "destack.display.window.raise",
        |binding| Ok(binding.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    backend_core::request_surface_activation(context, surface_id, "destack.display.window.raise")
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding =
        super::resolve_window_binding(context, window_handle, "destack.display.window.minimize")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = binding.visibility;
    if previous_visibility == WindowVisibility::Minimized {
        return Ok(());
    }

    // request compositor minimization
    let xdg_toplevel_id =
        super::require_xdg_toplevel_id(&binding, "destack.display.window.minimize")?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.minimize",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.minimize",
            )?;

            toplevel.set_minimized();
            backend_core::flush_queue(event_queue, "destack.display.window.minimize")?;

            Ok(())
        },
    )?;

    binding.visibility = WindowVisibility::Minimized;
    drop(binding);

    let runtime_state = backend_core::runtime_state(context);
    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        WindowVisibility::Minimized,
    );

    Ok(())
}

/// Maximize one window.
pub(crate) unsafe fn window_maximize(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding =
        super::resolve_window_binding(context, window_handle, "destack.display.window.maximize")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = binding.visibility;
    if previous_visibility == WindowVisibility::Maximized {
        return Ok(());
    }

    // request compositor maximization
    let xdg_toplevel_id =
        super::require_xdg_toplevel_id(&binding, "destack.display.window.maximize")?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.maximize",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.maximize",
            )?;

            toplevel.set_maximized();
            backend_core::flush_queue(event_queue, "destack.display.window.maximize")?;

            Ok(())
        },
    )?;

    binding.visibility = WindowVisibility::Maximized;
    drop(binding);

    let runtime_state = backend_core::runtime_state(context);
    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        WindowVisibility::Maximized,
    );

    Ok(())
}

/// Restore one window.
pub(crate) unsafe fn window_restore(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window binding and enforce owner-thread affinity
    let binding =
        super::resolve_window_binding(context, window_handle, "destack.display.window.restore")?;
    let mut binding = binding.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = binding.visibility;
    if previous_visibility == WindowVisibility::Visible {
        return Ok(());
    }

    // clear compositor fullscreen and maximize state
    let xdg_toplevel_id =
        super::require_xdg_toplevel_id(&binding, "destack.display.window.restore")?;
    backend_core::with_connection_dispatch(
        context,
        "destack.display.window.restore",
        |connection, event_queue, _dispatch_state| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.restore",
            )?;

            toplevel.unset_fullscreen();
            toplevel.unset_maximized();
            backend_core::flush_queue(event_queue, "destack.display.window.restore")?;

            Ok(())
        },
    )?;

    binding.visibility = WindowVisibility::Visible;
    drop(binding);

    let runtime_state = backend_core::runtime_state(context);
    event::publish_window_visibility_changed(
        &runtime_state,
        window_handle,
        previous_visibility,
        WindowVisibility::Visible,
    );

    Ok(())
}

/// Begin one native window move drag.
pub(crate) unsafe fn window_begin_move_drag(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve one target xdg_toplevel id from owner-thread binding
    let xdg_toplevel_id = super::with_window_binding(
        context,
        window_handle,
        "destack.display.window.beginMoveDrag",
        |binding| super::require_xdg_toplevel_id(binding, "destack.display.window.beginMoveDrag"),
    )?;

    // request compositor interactive move with seat and serial lanes
    backend_core::with_interaction_serial(
        context,
        "destack.display.window.beginMoveDrag",
        |connection, event_queue, _dispatch_state, seat, serial| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.beginMoveDrag",
            )?;

            toplevel._move(&seat, serial);
            backend_core::flush_queue(event_queue, "destack.display.window.beginMoveDrag")?;
            Ok(())
        },
    )
}

/// Begin one native window resize drag.
pub(crate) unsafe fn window_begin_resize_drag(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    edge: WindowResizeEdge,
) -> RuntimeResult<()> {
    // resolve target xdg_toplevel id from owner-thread binding
    let xdg_toplevel_id = super::with_window_binding(
        context,
        window_handle,
        "destack.display.window.beginResizeDrag",
        |binding| super::require_xdg_toplevel_id(binding, "destack.display.window.beginResizeDrag"),
    )?;

    // map abstract resize edge into xdg-shell resize edge enum
    let edge = match edge {
        WindowResizeEdge::North => xdg_toplevel::ResizeEdge::Top,
        WindowResizeEdge::South => xdg_toplevel::ResizeEdge::Bottom,
        WindowResizeEdge::West => xdg_toplevel::ResizeEdge::Left,
        WindowResizeEdge::East => xdg_toplevel::ResizeEdge::Right,
        WindowResizeEdge::NorthWest => xdg_toplevel::ResizeEdge::TopLeft,
        WindowResizeEdge::NorthEast => xdg_toplevel::ResizeEdge::TopRight,
        WindowResizeEdge::SouthWest => xdg_toplevel::ResizeEdge::BottomLeft,
        WindowResizeEdge::SouthEast => xdg_toplevel::ResizeEdge::BottomRight,
    };

    // request compositor interactive resize with seat and serial lanes
    backend_core::with_interaction_serial(
        context,
        "destack.display.window.beginResizeDrag",
        |connection, event_queue, _dispatch_state, seat, serial| {
            let toplevel = backend_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.beginResizeDrag",
            )?;

            toplevel.resize(&seat, serial, edge);
            backend_core::flush_queue(event_queue, "destack.display.window.beginResizeDrag")?;
            Ok(())
        },
    )
}
