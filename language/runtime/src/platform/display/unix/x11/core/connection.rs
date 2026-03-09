use std::sync::Arc;

use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::protocol::randr::{ConnectionExt as RandrConnectionExt, NotifyMask};
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as XprotoConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::core::BackendSupport;
use crate::platform::display::DisplayBackendCapabilityFlags;
use crate::platform::{PlatformError, display as display_platform};
use crate::runtime::BindingCallContext;

use super::core::{io_error, selected_backend_name};
use super::runtime::{X11RuntimeState, runtime_state};
use crate::platform::display::unix::x11::monitor;

/// Interned X11 atoms used by the runtime.
#[derive(Debug, Clone)]
pub(crate) struct X11Atoms {
    /// `WM_PROTOCOLS` atom.
    pub(crate) wm_protocols: Atom,
    /// `WM_DELETE_WINDOW` atom.
    pub(crate) wm_delete_window: Atom,
    /// `WM_CHANGE_STATE` atom.
    pub(crate) wm_change_state: Atom,
    /// `WM_STATE` atom.
    pub(crate) wm_state: Atom,
    /// `UTF8_STRING` atom.
    pub(crate) utf8_string: Atom,
    /// `WM_NAME` atom.
    pub(crate) wm_name: Atom,
    /// `_NET_WM_NAME` atom.
    pub(crate) net_wm_name: Atom,
    /// `_NET_WM_STATE` atom.
    pub(crate) net_wm_state: Atom,
    /// `_NET_WM_STATE_FULLSCREEN` atom.
    pub(crate) net_wm_state_fullscreen: Atom,
    /// `_NET_WM_STATE_MAXIMIZED_HORZ` atom.
    pub(crate) net_wm_state_maximized_horz: Atom,
    /// `_NET_WM_STATE_MAXIMIZED_VERT` atom.
    pub(crate) net_wm_state_maximized_vert: Atom,
    /// `_NET_WM_STATE_ABOVE` atom.
    pub(crate) net_wm_state_above: Atom,
    /// `_NET_WM_STATE_SKIP_TASKBAR` atom.
    pub(crate) net_wm_state_skip_taskbar: Atom,
    /// `_NET_WM_STATE_MODAL` atom.
    pub(crate) net_wm_state_modal: Atom,
    /// `_NET_WM_STATE_DEMANDS_ATTENTION` atom.
    pub(crate) net_wm_state_demands_attention: Atom,
    /// `_NET_WM_WINDOW_OPACITY` atom.
    pub(crate) net_wm_window_opacity: Atom,
    /// `_NET_WM_WINDOW_TYPE` atom.
    pub(crate) net_wm_window_type: Atom,
    /// `_NET_WM_WINDOW_TYPE_NORMAL` atom.
    pub(crate) net_wm_window_type_normal: Atom,
    /// `_NET_WM_WINDOW_TYPE_UTILITY` atom.
    pub(crate) net_wm_window_type_utility: Atom,
    /// `_NET_WM_WINDOW_TYPE_POPUP_MENU` atom.
    pub(crate) net_wm_window_type_popup_menu: Atom,
    /// `_MOTIF_WM_HINTS` atom.
    pub(crate) motif_wm_hints: Atom,
    /// `_NET_WM_MOVERESIZE` atom.
    pub(crate) net_wm_moveresize: Atom,
    /// `_NET_WM_ICON` atom.
    pub(crate) net_wm_icon: Atom,
    /// `_NET_WORKAREA` atom.
    pub(crate) net_work_area: Atom,
    /// `_NET_CURRENT_DESKTOP` atom.
    pub(crate) net_current_desktop: Atom,
    /// `vrr_capable` atom.
    pub(crate) vrr_capable: Atom,
    /// `XdndAware` atom.
    pub(crate) xdnd_aware: Atom,
    /// `XdndEnter` atom.
    pub(crate) xdnd_enter: Atom,
    /// `XdndPosition` atom.
    pub(crate) xdnd_position: Atom,
    /// `XdndStatus` atom.
    pub(crate) xdnd_status: Atom,
    /// `XdndDrop` atom.
    pub(crate) xdnd_drop: Atom,
    /// `XdndFinished` atom.
    pub(crate) xdnd_finished: Atom,
    /// `XdndLeave` atom.
    pub(crate) xdnd_leave: Atom,
    /// `XdndSelection` atom.
    pub(crate) xdnd_selection: Atom,
    /// `XdndTypeList` atom.
    pub(crate) xdnd_type_list: Atom,
    /// `XdndActionCopy` atom.
    pub(crate) xdnd_action_copy: Atom,
    /// `text/uri-list` atom.
    pub(crate) text_uri_list: Atom,
    /// `TEXT` atom.
    pub(crate) text: Atom,
}

/// Shared X11 host connection lane and root metadata.
#[derive(Debug, Clone)]
pub(crate) struct X11ConnectionState {
    /// Shared X11 connection for this runtime.
    pub(crate) connection: Arc<RustConnection>,
    /// Selected setup screen index.
    pub(crate) screen_index: usize,
    /// Root window for the selected screen.
    pub(crate) root: Window,
    /// Interned atoms for this connection.
    pub(crate) atoms: X11Atoms,
    /// Extension support flags for this host connection.
    pub(crate) extensions: X11ExtensionSupport,
}

/// Extension support state for one x11 connection.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct X11ExtensionSupport {
    /// Whether `RANDR` is available.
    pub(crate) randr: bool,
    /// Whether `XFIXES` is available.
    pub(crate) xfixes: bool,
    /// Whether `SHAPE` is available.
    pub(crate) shape: bool,
}

/// Return one interned atom value by name.
fn intern_atom(
    connection: &RustConnection,
    name: &[u8],
    operation: &'static str,
) -> RuntimeResult<Atom> {
    let cookie = connection
        .intern_atom(false, name)
        .map_err(|error| io_error(operation, format!("intern_atom request failed: {error}")))?;
    let reply = cookie
        .reply()
        .map_err(|error| io_error(operation, format!("intern_atom reply failed: {error}")))?;

    Ok(reply.atom)
}

/// Query extension support for one connected X11 host.
fn query_extension_support(
    connection: &RustConnection,
    operation: &'static str,
) -> RuntimeResult<X11ExtensionSupport> {
    Ok(X11ExtensionSupport {
        randr: query_extension_present(connection, b"RANDR", operation)?,
        xfixes: query_extension_present(connection, b"XFIXES", operation)?,
        shape: query_extension_present(connection, b"SHAPE", operation)?,
    })
}

/// Query whether one X11 extension is present.
fn query_extension_present(
    connection: &RustConnection,
    extension_name: &[u8],
    operation: &'static str,
) -> RuntimeResult<bool> {
    let cookie = connection
        .query_extension(extension_name)
        .map_err(|error| {
            io_error(
                operation,
                format!("query_extension request failed: {error}"),
            )
        })?;
    let reply = cookie
        .reply()
        .map_err(|error| io_error(operation, format!("query_extension reply failed: {error}")))?;

    Ok(reply.present)
}

/// Query whether one active CRTC exposes gamma-ramp control.
pub(crate) fn query_gamma_control_available(
    connection: &RustConnection,
    root: Window,
) -> RuntimeResult<bool> {
    let resources = match connection.randr_get_screen_resources_current(root) {
        Ok(cookie) => cookie.reply().map_err(|error| {
            io_error(
                "destack.display.backend.list",
                format!("randr_get_screen_resources_current reply failed: {error}"),
            )
        })?,
        Err(ConnectionError::UnsupportedExtension) => return Ok(false),
        Err(error) => {
            return Err(io_error(
                "destack.display.backend.list",
                format!("randr_get_screen_resources_current request failed: {error}"),
            ));
        }
    };

    // scan active crtcs for a non-zero gamma ramp size
    for crtc in resources.crtcs.iter().copied().filter(|crtc| *crtc != 0) {
        let gamma_size = connection
            .randr_get_crtc_gamma_size(crtc)
            .map_err(|error| {
                io_error(
                    "destack.display.backend.list",
                    format!("randr_get_crtc_gamma_size request failed: {error}"),
                )
            })?
            .reply()
            .map_err(|error| {
                io_error(
                    "destack.display.backend.list",
                    format!("randr_get_crtc_gamma_size reply failed: {error}"),
                )
            })?;

        // report gamma control once any active crtc exposes a ramp
        if gamma_size.size > 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Return one x11 connection snapshot, connecting lazily on first use.
pub(crate) fn connection_state(
    runtime_state: &Arc<X11RuntimeState>,
    operation: &'static str,
) -> RuntimeResult<Arc<X11ConnectionState>> {
    // return the cached connection state when already initialized
    {
        let state = runtime_state
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(connection_state) = state.as_ref() {
            return Ok(Arc::clone(connection_state));
        }
    }

    // reject when x11 display endpoint is unavailable
    if std::env::var_os("DISPLAY").is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(format!(
            "{operation}: DISPLAY is not set for {} backend",
            selected_backend_name(),
        )))
        .boxed());
    }

    // open one new x11 connection and intern required atoms
    let (connection, screen_index) = x11rb::connect(None).map_err(|error| {
        RuntimeError::from(PlatformError::not_supported(format!(
            "{operation}: {} connect failed: {error}",
            selected_backend_name(),
        )))
        .boxed()
    })?;
    let root = connection
        .setup()
        .roots
        .get(screen_index)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::generic(
                None,
                "x11 setup missing selected screen root",
            ))
            .boxed()
        })?
        .root;
    let connection = Arc::new(connection);
    let extensions = query_extension_support(connection.as_ref(), operation)?;

    // subscribe to randr topology-change events when the extension is present
    if extensions.randr {
        connection
            .randr_select_input(
                root,
                NotifyMask::SCREEN_CHANGE
                    | NotifyMask::CRTC_CHANGE
                    | NotifyMask::OUTPUT_CHANGE
                    | NotifyMask::PROVIDER_CHANGE
                    | NotifyMask::RESOURCE_CHANGE,
            )
            .map_err(|error| {
                RuntimeError::from(PlatformError::io_with(
                    None,
                    None,
                    None,
                    Some(format!("{operation}: randr_select_input")),
                    None,
                    format!("request failed: {error}"),
                ))
                .boxed()
            })?
            .check()
            .map_err(|error| {
                RuntimeError::from(PlatformError::io_with(
                    None,
                    None,
                    None,
                    Some(format!("{operation}: randr_select_input")),
                    None,
                    format!("reply failed: {error}"),
                ))
                .boxed()
            })?;
    }

    let atoms = X11Atoms {
        wm_protocols: intern_atom(&connection, b"WM_PROTOCOLS", operation)?,
        wm_delete_window: intern_atom(&connection, b"WM_DELETE_WINDOW", operation)?,
        wm_change_state: intern_atom(&connection, b"WM_CHANGE_STATE", operation)?,
        wm_state: intern_atom(&connection, b"WM_STATE", operation)?,
        utf8_string: intern_atom(&connection, b"UTF8_STRING", operation)?,
        wm_name: AtomEnum::WM_NAME.into(),
        net_wm_name: intern_atom(&connection, b"_NET_WM_NAME", operation)?,
        net_wm_state: intern_atom(&connection, b"_NET_WM_STATE", operation)?,
        net_wm_state_fullscreen: intern_atom(&connection, b"_NET_WM_STATE_FULLSCREEN", operation)?,
        net_wm_state_maximized_horz: intern_atom(
            &connection,
            b"_NET_WM_STATE_MAXIMIZED_HORZ",
            operation,
        )?,
        net_wm_state_maximized_vert: intern_atom(
            &connection,
            b"_NET_WM_STATE_MAXIMIZED_VERT",
            operation,
        )?,
        net_wm_state_above: intern_atom(&connection, b"_NET_WM_STATE_ABOVE", operation)?,
        net_wm_state_skip_taskbar: intern_atom(
            &connection,
            b"_NET_WM_STATE_SKIP_TASKBAR",
            operation,
        )?,
        net_wm_state_modal: intern_atom(&connection, b"_NET_WM_STATE_MODAL", operation)?,
        net_wm_state_demands_attention: intern_atom(
            &connection,
            b"_NET_WM_STATE_DEMANDS_ATTENTION",
            operation,
        )?,
        net_wm_window_opacity: intern_atom(&connection, b"_NET_WM_WINDOW_OPACITY", operation)?,
        net_wm_window_type: intern_atom(&connection, b"_NET_WM_WINDOW_TYPE", operation)?,
        net_wm_window_type_normal: intern_atom(
            &connection,
            b"_NET_WM_WINDOW_TYPE_NORMAL",
            operation,
        )?,
        net_wm_window_type_utility: intern_atom(
            &connection,
            b"_NET_WM_WINDOW_TYPE_UTILITY",
            operation,
        )?,
        net_wm_window_type_popup_menu: intern_atom(
            &connection,
            b"_NET_WM_WINDOW_TYPE_POPUP_MENU",
            operation,
        )?,
        motif_wm_hints: intern_atom(&connection, b"_MOTIF_WM_HINTS", operation)?,
        net_wm_moveresize: intern_atom(&connection, b"_NET_WM_MOVERESIZE", operation)?,
        net_wm_icon: intern_atom(&connection, b"_NET_WM_ICON", operation)?,
        net_work_area: intern_atom(&connection, b"_NET_WORKAREA", operation)?,
        net_current_desktop: intern_atom(&connection, b"_NET_CURRENT_DESKTOP", operation)?,
        vrr_capable: intern_atom(&connection, b"vrr_capable", operation)?,
        xdnd_aware: intern_atom(&connection, b"XdndAware", operation)?,
        xdnd_enter: intern_atom(&connection, b"XdndEnter", operation)?,
        xdnd_position: intern_atom(&connection, b"XdndPosition", operation)?,
        xdnd_status: intern_atom(&connection, b"XdndStatus", operation)?,
        xdnd_drop: intern_atom(&connection, b"XdndDrop", operation)?,
        xdnd_finished: intern_atom(&connection, b"XdndFinished", operation)?,
        xdnd_leave: intern_atom(&connection, b"XdndLeave", operation)?,
        xdnd_selection: intern_atom(&connection, b"XdndSelection", operation)?,
        xdnd_type_list: intern_atom(&connection, b"XdndTypeList", operation)?,
        xdnd_action_copy: intern_atom(&connection, b"XdndActionCopy", operation)?,
        text_uri_list: intern_atom(&connection, b"text/uri-list", operation)?,
        text: intern_atom(&connection, b"TEXT", operation)?,
    };

    let snapshot = Arc::new(X11ConnectionState {
        connection,
        screen_index,
        root,
        atoms,
        extensions,
    });

    // cache and return the initialized connection state
    let mut state = runtime_state
        .connection
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if let Some(existing) = state.as_ref() {
        return Ok(Arc::clone(existing));
    }
    *state = Some(Arc::clone(&snapshot));

    Ok(snapshot)
}

/// Return backend descriptor support and capability flags for x11.
pub(crate) fn backend_descriptor_state(
    binding: &BindingCallContext,
) -> (BackendSupport, DisplayBackendCapabilityFlags) {
    // treat x11 as unavailable when no display endpoint is configured
    if std::env::var_os("DISPLAY").is_none() {
        return (
            BackendSupport::HostUnavailable,
            DisplayBackendCapabilityFlags(0),
        );
    }

    // resolve one shared x11 connection for capability probing
    let runtime_state = runtime_state(binding);
    let connection_state = match connection_state(&runtime_state, "destack.display.backend.list") {
        Ok(value) => value,
        Err(_) => {
            return (
                BackendSupport::HostUnavailable,
                DisplayBackendCapabilityFlags(0),
            );
        }
    };

    // publish baseline capability lanes supported without optional extensions
    let mut capability_flags = display_platform::DISPLAY_BACKEND_CAP_WINDOW.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_STATE.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_MONITOR_EVENTS.0
        | display_platform::DISPLAY_BACKEND_CAP_EXCLUSIVE_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_BORDERLESS_FULLSCREEN.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_LOCK.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_CONFINE.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_WARP.0
        | display_platform::DISPLAY_BACKEND_CAP_CURSOR_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_TRANSPARENCY.0
        | display_platform::DISPLAY_BACKEND_CAP_ALWAYS_ON_TOP.0
        | display_platform::DISPLAY_BACKEND_CAP_ATTENTION_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_REFRESH_REQUEST.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ICON.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_OPACITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_FOCUS.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_RAISE.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DRAG_INTERACTION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_PARENTING.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_MODAL.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ASPECT_RATIO.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_CHROME.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_TASKBAR_VISIBILITY.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_POPUP.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_ROLE_OVERLAY.0
        | display_platform::DISPLAY_BACKEND_CAP_OCCLUSION.0
        | display_platform::DISPLAY_BACKEND_CAP_WINDOW_DROP_EVENTS.0;

    // enable cursor visibility lane when xfixes is present
    if connection_state.extensions.xfixes {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_CURSOR_VISIBILITY.0;
    }

    // enable monitor gamma control lane when randr is present
    if connection_state.extensions.randr {
        let mode_set_available =
            monitor::monitor_mode_set_supported(connection_state.as_ref()).unwrap_or(false);
        if mode_set_available {
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
        }

        let gamma_available = query_gamma_control_available(
            connection_state.connection.as_ref(),
            connection_state.root,
        )
        .unwrap_or(false);
        if gamma_available {
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
        }
    }

    // enable hit-test lane when shape extension is present
    if connection_state.extensions.shape {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
    }

    (
        BackendSupport::Available,
        DisplayBackendCapabilityFlags(capability_flags),
    )
}
