use crate::platform::display::{self as display_platform, DisplayBackendCapabilityFlags};
use crate::runtime::BindingCallContext;

use super::connection_state;
use crate::platform::display::unix::wayland::monitor;

/// Return backend descriptor availability and capability flags for wayland.
pub(crate) fn backend_descriptor_state(
    context: &BindingCallContext,
) -> (bool, DisplayBackendCapabilityFlags) {
    // reject when no wayland endpoint is configured
    if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("WAYLAND_SOCKET").is_none()
    {
        return (false, DisplayBackendCapabilityFlags(0));
    }

    // reject when connection and monitor enumeration are unavailable
    let connection_state = connection_state(context, "destack.display.backend.list");
    let available = connection_state
        .as_ref()
        .map(|_| true)
        .or_else(|_| monitor::enumerate_monitor_snapshots(context).map(|value| !value.is_empty()))
        .unwrap_or(false);
    if !available {
        return (false, DisplayBackendCapabilityFlags(0));
    }

    // load optional protocol support from resolved wayland globals
    let (
        supports_window_decorations,
        supports_window_modal,
        supports_window_activation,
        supports_window_icon,
        supports_window_opacity,
        supports_window_drop_events,
        supports_monitor_mode_set,
        supports_monitor_color_state,
        supports_monitor_gamma_control,
        supports_cursor_shape,
        supports_cursor_lock,
        supports_cursor_confine,
        supports_cursor_warp,
        supports_window_drag_interaction,
        supports_window_role_popup,
        supports_window_role_overlay,
    ) = if let Ok(connection_state) = connection_state {
        let dispatch_state = connection_state
            .dispatch_state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        (
            dispatch_state.supports_window_decorations(),
            dispatch_state.supports_window_modal(),
            dispatch_state.supports_window_activation(),
            dispatch_state.supports_window_icon(),
            dispatch_state.supports_window_opacity(),
            dispatch_state.supports_window_drop_events(),
            dispatch_state.supports_monitor_mode_set(),
            dispatch_state.supports_monitor_color_state(),
            dispatch_state.supports_monitor_gamma_control(),
            dispatch_state.supports_cursor_shape(),
            dispatch_state.supports_cursor_lock(),
            dispatch_state.supports_cursor_confine(),
            dispatch_state.supports_cursor_warp(),
            dispatch_state.supports_window_drag_interaction(),
            dispatch_state.supports_window_role_popup(),
            dispatch_state.supports_window_role_overlay(),
        )
    } else {
        (
            false, false, false, false, false, false, false, false, false, false, false, false,
            false, false, false, false,
        )
    };

    // report only lanes that are meaningfully protocol-backed in this backend
    let mut capability_flags = display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_REFRESH_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
    // monitor hdr control remains disabled: generic wayland output-policy writes are unavailable
    let supports_monitor_hdr_control = false;

    // add optional lanes negotiated through extension globals
    if supports_window_decorations {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0;
    }

    if supports_window_modal {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0;
    }

    if supports_window_activation {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
            | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0;
    }

    if supports_window_icon {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0;
    }

    if supports_window_opacity {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0;
    }

    if supports_window_drop_events {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0;
    }

    if supports_monitor_mode_set {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
    }

    if supports_monitor_color_state {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
    }

    if supports_monitor_hdr_control {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_HDR_CONTROL.0;
    }

    if supports_monitor_gamma_control {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
    }

    if supports_cursor_shape {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
            | display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0;
    }

    if supports_cursor_lock {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0;
    }

    if supports_cursor_confine {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0;
    }

    if supports_cursor_warp {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0;
    }

    if supports_window_drag_interaction {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0;
    }

    if supports_window_role_popup {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0;
    }

    if supports_window_role_overlay {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0;
    }

    (true, DisplayBackendCapabilityFlags(capability_flags))
}
