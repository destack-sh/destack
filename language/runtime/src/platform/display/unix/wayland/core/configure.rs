use crate::platform::display::{
    WindowLogicalSize, WindowPhysicalSize, WindowPosition, WindowVisibility,
};

use super::{
    WaylandConnectionDispatchState, WaylandWindowDispatchToken, XDG_TOPLEVEL_STATE_ACTIVATED,
    XDG_TOPLEVEL_STATE_FULLSCREEN, XDG_TOPLEVEL_STATE_MAXIMIZED,
};
use crate::platform::display::unix::wayland::event;

/// Decode xdg_toplevel state flags from one raw state-array payload.
fn decode_toplevel_states(states: &[u8]) -> (bool, bool, bool) {
    let mut is_maximized = false;
    let mut is_fullscreen = false;
    let mut is_activated = false;

    // parse one u32 sequence from the packed wayland state array
    for lane in states.chunks_exact(4) {
        let lane = u32::from_ne_bytes([lane[0], lane[1], lane[2], lane[3]]);

        if lane == XDG_TOPLEVEL_STATE_MAXIMIZED {
            is_maximized = true;
        }
        if lane == XDG_TOPLEVEL_STATE_FULLSCREEN {
            is_fullscreen = true;
        }
        if lane == XDG_TOPLEVEL_STATE_ACTIVATED {
            is_activated = true;
        }
    }

    (is_maximized, is_fullscreen, is_activated)
}

/// Apply one toplevel configure event to runtime window state and events.
pub(crate) fn apply_toplevel_configure(
    dispatch_state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
    width: i32,
    height: i32,
    states: &[u8],
) {
    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        return;
    };
    let Some(host_state) = token.host_state.upgrade() else {
        return;
    };
    let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
        return;
    };

    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // ignore configure events for already-destroyed host states
    if host_state.destroyed_emitted {
        return;
    }

    // update visibility from maximize and fullscreen state lanes
    let (is_maximized, is_fullscreen, is_activated) = decode_toplevel_states(states);
    let next_visibility = if is_maximized {
        WindowVisibility::Maximized
    } else if is_fullscreen {
        WindowVisibility::Visible
    } else {
        WindowVisibility::Visible
    };

    if host_state.visibility != next_visibility
        && host_state.visibility != WindowVisibility::Hidden
        && host_state.visibility != WindowVisibility::Minimized
    {
        host_state.visibility = next_visibility;
    }

    // update focus state from activated lane
    if host_state.focused != is_activated {
        host_state.focused = is_activated;
    }

    // update logical and physical sizes when compositor reports explicit dimensions
    if width > 0 && height > 0 {
        let current_size_physical = WindowPhysicalSize {
            width: width as u32,
            height: height as u32,
        };

        if host_state.size_physical != current_size_physical {
            let previous_size_logical = host_state.size_logical;
            let previous_size_physical = host_state.size_physical;

            let scale = if host_state.scale_factor_milli == 0 {
                1.0
            } else {
                host_state.scale_factor_milli as f64 / 1000.0
            };
            let current_size_logical = WindowLogicalSize {
                width: (current_size_physical.width as f64 / scale).max(1.0),
                height: (current_size_physical.height as f64 / scale).max(1.0),
            };

            host_state.size_logical = current_size_logical;
            host_state.size_physical = current_size_physical;
        }
    }

    let current = host_state.clone();
    drop(host_state);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);
}

/// Apply one compositor close request to runtime window event publication.
pub(crate) fn apply_toplevel_close(
    dispatch_state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
) {
    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        return;
    };
    let Some(host_state) = token.host_state.upgrade() else {
        return;
    };
    let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
        return;
    };

    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());

    // emit close-requested only once per window lifetime
    if host_state.close_requested_emitted {
        return;
    }

    host_state.close_requested_emitted = true;
    drop(host_state);

    event::publish_window_close_requested(&runtime_state, window_handle);
}

/// Apply one popup configure event to runtime window state.
pub(crate) fn apply_popup_configure(
    dispatch_state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        return;
    };
    let Some(host_state) = token.host_state.upgrade() else {
        return;
    };
    let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
        return;
    };

    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // ignore configure events for already-destroyed host states
    if host_state.destroyed_emitted {
        return;
    }

    let previous_position = host_state.position;
    let current_position = WindowPosition { x, y };
    host_state.position = current_position;

    let mut size_change = None;

    // update runtime size snapshots when compositor provided positive dimensions
    if width > 0 && height > 0 {
        let next_width = width as u32;
        let next_height = height as u32;
        let previous_size_physical = host_state.size_physical;
        let next_size_physical = WindowPhysicalSize {
            width: next_width,
            height: next_height,
        };

        if previous_size_physical != next_size_physical {
            let scale_factor_milli = host_state.scale_factor_milli.max(1);
            let scale_factor = scale_factor_milli as f64 / 1000.0;
            let previous_size_logical = host_state.size_logical;
            let next_size_logical = WindowLogicalSize {
                width: next_width as f64 / scale_factor,
                height: next_height as f64 / scale_factor,
            };

            host_state.size_physical = next_size_physical;
            host_state.size_logical = next_size_logical;

            size_change = Some((
                previous_size_logical,
                previous_size_physical,
                next_size_logical,
                next_size_physical,
            ));
        }
    }

    let current = host_state.clone();
    drop(host_state);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);

    if previous_position != current_position {
        event::publish_window_refresh_requested(&runtime_state, window_handle);
    }
}

/// Apply one layer-surface configure event to runtime window state.
pub(crate) fn apply_layer_surface_configure(
    dispatch_state: &WaylandConnectionDispatchState,
    token: &WaylandWindowDispatchToken,
    width: u32,
    height: u32,
) {
    let Some(runtime_state) = dispatch_state.runtime_state.upgrade() else {
        return;
    };
    let Some(host_state) = token.host_state.upgrade() else {
        return;
    };
    let Some(window_handle) = runtime_state.window_handle_from_id(&token.window_id) else {
        return;
    };

    let mut host_state = host_state.lock().unwrap_or_else(|error| error.into_inner());
    let previous = host_state.clone();

    // ignore configure events for already-destroyed host states
    if host_state.destroyed_emitted {
        return;
    }

    // ignore compositor configure payloads that defer size selection
    if width == 0 || height == 0 {
        return;
    }

    let previous_size_physical = host_state.size_physical;
    let next_size_physical = WindowPhysicalSize { width, height };
    if previous_size_physical == next_size_physical {
        return;
    }

    let scale_factor_milli = host_state.scale_factor_milli.max(1);
    let scale_factor = scale_factor_milli as f64 / 1000.0;
    let previous_size_logical = host_state.size_logical;
    let next_size_logical = WindowLogicalSize {
        width: width as f64 / scale_factor,
        height: height as f64 / scale_factor,
    };

    host_state.size_physical = next_size_physical;
    host_state.size_logical = next_size_logical;
    let current = host_state.clone();
    drop(host_state);
    event::publish_state_deltas(&runtime_state, window_handle, &previous, &current);
}
