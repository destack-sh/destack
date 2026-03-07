use crate::diagnostic::RuntimeResult;
use crate::platform::display::{WindowAttentionLevel, WindowResizeEdge, WindowVisibility};
use crate::platform::resource::WindowHandle;
use crate::runtime::BindingCallContext;
use wayland_protocols::xdg::shell::client::xdg_toplevel;

use super::{require_xdg_toplevel_id, resolve_window_host_state, with_window_host_state};
use crate::platform::display::unix::wayland::{core as wayland_core, event};

/// Request one user-attention pulse for one window.
pub(crate) unsafe fn window_request_attention(
    context: &BindingCallContext,
    window_handle: WindowHandle,
    _level: WindowAttentionLevel,
) -> RuntimeResult<()> {
    // resolve target surface from one live window host state
    let surface_id = with_window_host_state(
        context,
        window_handle,
        "destack.display.window.requestAttention",
        |host_state| Ok(host_state.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    wayland_core::request_surface_activation(
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
    // validate one live window host state
    resolve_window_host_state(
        context,
        window_handle,
        "destack.display.window.requestRefresh",
    )?;

    // publish refresh-requested event
    let runtime_state = wayland_core::runtime_state(context);
    event::publish_window_refresh_requested(&runtime_state, window_handle);

    Ok(())
}

/// Focus one window.
pub(crate) unsafe fn window_focus(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target surface from one live window host state
    let surface_id = with_window_host_state(
        context,
        window_handle,
        "destack.display.window.focus",
        |host_state| Ok(host_state.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    wayland_core::request_surface_activation(context, surface_id, "destack.display.window.focus")
}

/// Raise one window.
pub(crate) unsafe fn window_raise(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target surface from one live window host state
    let surface_id = with_window_host_state(
        context,
        window_handle,
        "destack.display.window.raise",
        |host_state| Ok(host_state.host.surface.clone()),
    )?;

    // request compositor activation through xdg-activation
    wayland_core::request_surface_activation(context, surface_id, "destack.display.window.raise")
}

/// Minimize one window.
pub(crate) unsafe fn window_minimize(
    context: &BindingCallContext,
    window_handle: WindowHandle,
) -> RuntimeResult<()> {
    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.minimize")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = host_state.visibility;
    if previous_visibility == WindowVisibility::Minimized {
        return Ok(());
    }

    // request compositor minimization
    let xdg_toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.minimize")?;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.minimize",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.minimize",
            )?;

            toplevel.set_minimized();
            wayland_core::flush_queue(event_queue, "destack.display.window.minimize")?;

            Ok(())
        },
    )?;

    host_state.visibility = WindowVisibility::Minimized;
    drop(host_state);

    let runtime_state = wayland_core::runtime_state(context);
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
    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.maximize")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = host_state.visibility;
    if previous_visibility == WindowVisibility::Maximized {
        return Ok(());
    }

    // request compositor maximization
    let xdg_toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.maximize")?;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.maximize",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.maximize",
            )?;

            toplevel.set_maximized();
            wayland_core::flush_queue(event_queue, "destack.display.window.maximize")?;

            Ok(())
        },
    )?;

    host_state.visibility = WindowVisibility::Maximized;
    drop(host_state);

    let runtime_state = wayland_core::runtime_state(context);
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
    // resolve target window host state and mutate host state
    let host_state =
        resolve_window_host_state(context, window_handle, "destack.display.window.restore")?;
    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    let previous_visibility = host_state.visibility;
    if previous_visibility == WindowVisibility::Visible {
        return Ok(());
    }

    // clear compositor fullscreen and maximize state
    let xdg_toplevel_id = require_xdg_toplevel_id(&host_state, "destack.display.window.restore")?;
    wayland_core::with_connection_dispatch(
        context,
        "destack.display.window.restore",
        |connection, event_queue, _dispatch_state| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.restore",
            )?;

            toplevel.unset_fullscreen();
            toplevel.unset_maximized();
            wayland_core::flush_queue(event_queue, "destack.display.window.restore")?;

            Ok(())
        },
    )?;

    host_state.visibility = WindowVisibility::Visible;
    drop(host_state);

    let runtime_state = wayland_core::runtime_state(context);
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
    // resolve target xdg_toplevel id from window host state
    let xdg_toplevel_id = with_window_host_state(
        context,
        window_handle,
        "destack.display.window.beginMoveDrag",
        |host_state| require_xdg_toplevel_id(host_state, "destack.display.window.beginMoveDrag"),
    )?;

    // request compositor interactive move with seat and serial lanes
    wayland_core::with_interaction_serial(
        context,
        "destack.display.window.beginMoveDrag",
        |connection, event_queue, _dispatch_state, seat, serial| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.beginMoveDrag",
            )?;

            toplevel._move(&seat, serial);
            wayland_core::flush_queue(event_queue, "destack.display.window.beginMoveDrag")?;
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
    // resolve target xdg_toplevel id from window host state
    let xdg_toplevel_id = with_window_host_state(
        context,
        window_handle,
        "destack.display.window.beginResizeDrag",
        |host_state| require_xdg_toplevel_id(host_state, "destack.display.window.beginResizeDrag"),
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
    wayland_core::with_interaction_serial(
        context,
        "destack.display.window.beginResizeDrag",
        |connection, event_queue, _dispatch_state, seat, serial| {
            let toplevel = wayland_core::resolve_xdg_toplevel(
                connection,
                xdg_toplevel_id,
                "destack.display.window.beginResizeDrag",
            )?;

            toplevel.resize(&seat, serial, edge);
            wayland_core::flush_queue(event_queue, "destack.display.window.beginResizeDrag")?;
            Ok(())
        },
    )
}
