use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use x11rb::connection::Connection;
use x11rb::errors::ConnectionError;
use x11rb::protocol::randr::ConnectionExt as RandrConnectionExt;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as XprotoConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::{
    DisplayBackend, DisplayBackendCapabilityFlags, DisplayEventOverflowPolicy,
    DisplayMonitorEventKindMask, WindowEventKindMask,
};
use crate::platform::{
    PlatformError, core as core_platform, display as display_platform, resource,
};
use crate::runtime::BindingCallContext;

pub(super) use super::constants::*;
use super::event::{MonitorEventBinding, WindowEventBinding};
use super::model::MonitorSnapshot;
use super::monitor;

/// Return the backend for the x11 backend implementation.
pub(crate) fn selected_backend() -> DisplayBackend {
    DisplayBackend::X11
}

/// Return one backend label for x11 diagnostics and stable identifiers.
pub(crate) fn selected_backend_name() -> &'static str {
    "x11"
}

/// Interned X11 atoms used by the runtime.
#[derive(Debug, Clone)]
pub(super) struct X11Atoms {
    /// `WM_PROTOCOLS` atom.
    pub(super) wm_protocols: Atom,
    /// `WM_DELETE_WINDOW` atom.
    pub(super) wm_delete_window: Atom,
    /// `WM_CHANGE_STATE` atom.
    pub(super) wm_change_state: Atom,
    /// `WM_STATE` atom.
    pub(super) wm_state: Atom,
    /// `UTF8_STRING` atom.
    pub(super) utf8_string: Atom,
    /// `WM_NAME` atom.
    pub(super) wm_name: Atom,
    /// `_NET_WM_NAME` atom.
    pub(super) net_wm_name: Atom,
    /// `_NET_WM_STATE` atom.
    pub(super) net_wm_state: Atom,
    /// `_NET_WM_STATE_FULLSCREEN` atom.
    pub(super) net_wm_state_fullscreen: Atom,
    /// `_NET_WM_STATE_MAXIMIZED_HORZ` atom.
    pub(super) net_wm_state_maximized_horz: Atom,
    /// `_NET_WM_STATE_MAXIMIZED_VERT` atom.
    pub(super) net_wm_state_maximized_vert: Atom,
    /// `_NET_WM_STATE_ABOVE` atom.
    pub(super) net_wm_state_above: Atom,
    /// `_NET_WM_STATE_SKIP_TASKBAR` atom.
    pub(super) net_wm_state_skip_taskbar: Atom,
    /// `_NET_WM_STATE_MODAL` atom.
    pub(super) net_wm_state_modal: Atom,
    /// `_NET_WM_STATE_DEMANDS_ATTENTION` atom.
    pub(super) net_wm_state_demands_attention: Atom,
    /// `_NET_WM_WINDOW_OPACITY` atom.
    pub(super) net_wm_window_opacity: Atom,
    /// `_NET_WM_WINDOW_TYPE` atom.
    pub(super) net_wm_window_type: Atom,
    /// `_NET_WM_WINDOW_TYPE_NORMAL` atom.
    pub(super) net_wm_window_type_normal: Atom,
    /// `_NET_WM_WINDOW_TYPE_UTILITY` atom.
    pub(super) net_wm_window_type_utility: Atom,
    /// `_NET_WM_WINDOW_TYPE_POPUP_MENU` atom.
    pub(super) net_wm_window_type_popup_menu: Atom,
    /// `_MOTIF_WM_HINTS` atom.
    pub(super) motif_wm_hints: Atom,
    /// `_NET_WM_MOVERESIZE` atom.
    pub(super) net_wm_moveresize: Atom,
    /// `_NET_WM_ICON` atom.
    pub(super) net_wm_icon: Atom,
    /// `_NET_WORKAREA` atom.
    pub(super) net_work_area: Atom,
    /// `_NET_CURRENT_DESKTOP` atom.
    pub(super) net_current_desktop: Atom,
    /// `vrr_capable` atom.
    pub(super) vrr_capable: Atom,
    /// `XdndAware` atom.
    pub(super) xdnd_aware: Atom,
    /// `XdndEnter` atom.
    pub(super) xdnd_enter: Atom,
    /// `XdndPosition` atom.
    pub(super) xdnd_position: Atom,
    /// `XdndStatus` atom.
    pub(super) xdnd_status: Atom,
    /// `XdndDrop` atom.
    pub(super) xdnd_drop: Atom,
    /// `XdndFinished` atom.
    pub(super) xdnd_finished: Atom,
    /// `XdndLeave` atom.
    pub(super) xdnd_leave: Atom,
    /// `XdndSelection` atom.
    pub(super) xdnd_selection: Atom,
    /// `XdndTypeList` atom.
    pub(super) xdnd_type_list: Atom,
    /// `XdndActionCopy` atom.
    pub(super) xdnd_action_copy: Atom,
    /// `text/uri-list` atom.
    pub(super) text_uri_list: Atom,
    /// `TEXT` atom.
    pub(super) text: Atom,
}

/// Shared X11 host connection lane and root metadata.
#[derive(Debug, Clone)]
pub(super) struct X11ConnectionState {
    /// Shared X11 connection for this runtime.
    pub(super) connection: Arc<RustConnection>,
    /// Selected setup screen index.
    pub(super) screen_index: usize,
    /// Root window for the selected screen.
    pub(super) root: Window,
    /// Interned atoms for this connection.
    pub(super) atoms: X11Atoms,
    /// Extension support flags for this host connection.
    pub(super) extensions: X11ExtensionSupport,
}

/// Extension support state for one x11 connection.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct X11ExtensionSupport {
    /// Whether `RANDR` is available.
    pub(super) randr: bool,
    /// Whether `XFIXES` is available.
    pub(super) xfixes: bool,
    /// Whether `SHAPE` is available.
    pub(super) shape: bool,
}

/// Runtime-owned X11 display backend state.
pub(crate) struct X11RuntimeState {
    /// Lazy X11 connection state.
    connection: Mutex<Option<Arc<X11ConnectionState>>>,
    /// Monitor-event subscribers for this runtime.
    pub(super) monitor_event_registry: Mutex<Vec<Weak<MonitorEventBinding>>>,
    /// Window-event subscribers for this runtime.
    pub(super) window_event_registry: Mutex<Vec<Weak<WindowEventBinding>>>,
    /// Mapping from X11 window id to runtime window handle.
    pub(super) windows_by_xid: Mutex<HashMap<u32, resource::WindowHandle>>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(super) monitor_topology_snapshot: Mutex<Option<Vec<MonitorSnapshot>>>,
}

impl std::fmt::Debug for X11RuntimeState {
    /// Format this runtime state for diagnostics.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("X11RuntimeState")
            .finish_non_exhaustive()
    }
}

impl X11RuntimeState {
    /// Create one runtime-owned x11 state value.
    pub(super) fn from_context(binding: &BindingCallContext) -> Self {
        Self {
            connection: Mutex::new(None),
            monitor_event_registry: Mutex::new(Vec::new()),
            window_event_registry: Mutex::new(Vec::new()),
            windows_by_xid: Mutex::new(HashMap::new()),
            monitor_topology_snapshot: Mutex::new(None),
        }
    }
}

/// Return runtime-owned x11 state for this binding call.
pub(super) fn runtime_state(binding: &BindingCallContext) -> Arc<X11RuntimeState> {
    binding
        .agent()
        .platform_state
        .display
        .x11_runtime_state(|| X11RuntimeState::from_context(binding))
}

/// Return one x11 connection snapshot, connecting lazily on first use.
pub(super) fn connection_state(
    runtime_state: &Arc<X11RuntimeState>,
    operation: &'static str,
) -> RuntimeResult<Arc<X11ConnectionState>> {
    // return the cached connection state when already initialized
    {
        let state = runtime_state
            .connection
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        // evaluate this condition
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
    // evaluate this condition
    if let Some(existing) = state.as_ref() {
        return Ok(Arc::clone(existing));
    }
    *state = Some(Arc::clone(&snapshot));

    Ok(snapshot)
}

/// Return backend descriptor availability and capability flags for x11.
pub(crate) fn backend_descriptor_state(
    binding: &BindingCallContext,
) -> (bool, DisplayBackendCapabilityFlags) {
    // treat x11 as unavailable when no display endpoint is configured
    if std::env::var_os("DISPLAY").is_none() {
        return (false, DisplayBackendCapabilityFlags(0));
    }

    // resolve one shared x11 connection for capability probing
    let runtime_state = runtime_state(binding);
    let connection_state = match connection_state(&runtime_state, "destack.display.backend.list") {
        Ok(value) => value,
        Err(_) => return (false, DisplayBackendCapabilityFlags(0)),
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
        // evaluate this condition
        if mode_set_available {
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_MODE_SET.0;
        }

        let gamma_available = query_gamma_control_available(
            connection_state.connection.as_ref(),
            connection_state.root,
        )
        .unwrap_or(false);
        // evaluate this condition
        if gamma_available {
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_COLOR_STATE.0;
            capability_flags |= display_platform::DISPLAY_BACKEND_CAP_MONITOR_GAMMA_CONTROL.0;
        }
    }

    // enable hit-test lane when shape extension is present
    if connection_state.extensions.shape {
        capability_flags |= display_platform::DISPLAY_BACKEND_CAP_WINDOW_HIT_TEST.0;
    }

    (true, DisplayBackendCapabilityFlags(capability_flags))
}

/// Resolve one queue capacity from open options and runtime defaults.
pub(super) fn resolved_queue_capacity(binding: &BindingCallContext, requested: u32) -> usize {
    // use runtime default when request value is zero
    if requested == 0 {
        let configured = binding.agent().options.display.default_event_queue_capacity;
        return core_platform::option_u64_to_usize_or_min(
            configured,
            DEFAULT_EVENT_QUEUE_CAPACITY,
            1,
        );
    }

    requested as usize
}

/// Resolve one wait-slice interval for blocking window-event reads.
pub(super) fn window_event_wait_slice_ns(binding: &BindingCallContext) -> u64 {
    // use runtime override when present
    let configured = binding.agent().options.display.window_event_wait_slice_ns;

    // enforce one positive wait-slice value
    core_platform::option_u64_or_min(configured, DEFAULT_EVENT_WAIT_SLICE_NS, 1)
}

/// Validate one monitor-event kind mask.
pub(super) fn monitor_kind_mask(value: Option<DisplayMonitorEventKindMask>) -> u32 {
    value.map_or(DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one window-event kind mask.
pub(super) fn window_kind_mask(value: Option<WindowEventKindMask>) -> u64 {
    value.map_or(WINDOW_EVENT_KIND_MASK_ALL, |value| value.0)
}

/// Validate one monitor-event kind-mask payload.
pub(super) fn validate_monitor_event_kind_mask(
    kind_mask: u32,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !DISPLAY_MONITOR_EVENT_KIND_MASK_ALL;
    // evaluate this condition
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported monitor event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Validate one window-event kind-mask payload.
pub(super) fn validate_window_event_kind_mask(
    kind_mask: u64,
    field: &'static str,
) -> RuntimeResult<()> {
    let unsupported_bits = kind_mask & !WINDOW_EVENT_KIND_MASK_ALL;
    // evaluate this condition
    if unsupported_bits == 0 {
        return Ok(());
    }

    Err(core_platform::invalid_argument(
        field,
        format!("unsupported window event kind bits: 0x{unsupported_bits:x}"),
    ))
}

/// Build one busy error for queued-event overflow with `Error` policy.
pub(super) fn overflow_error(operation: &'static str) -> Box<RuntimeError> {
    core_platform::io_busy(
        operation,
        "event queue overflowed while overflow policy is error",
    )
}

/// Build one not-found error for missing window handles.
pub(super) fn window_not_found(
    operation: &'static str,
    window: resource::WindowHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("window handle {} was not found", window.0.0),
    )
}

/// Build one not-found error for missing display handles.
pub(super) fn display_not_found(
    operation: &'static str,
    handle: resource::DisplayHandle,
) -> Box<RuntimeError> {
    core_platform::io_not_found(
        operation,
        format!("display handle {} was not found", handle.0.0),
    )
}

/// Build one I/O error for x11 call failures.
pub(super) fn io_error(operation: &'static str, message: impl Into<String>) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        None,
        None,
        None,
        Some(operation.to_string()),
        None,
        message.into(),
    ))
    .boxed()
}

/// Push one event into one queue under overflow policy.
pub(super) fn push_with_overflow<T>(
    queue: &mut std::collections::VecDeque<T>,
    queue_capacity: usize,
    overflow_policy: DisplayEventOverflowPolicy,
    overflow_error_pending: &mut bool,
    dropped_count: &mut u64,
    value: T,
) {
    // append directly while capacity remains
    if queue.len() < queue_capacity {
        queue.push_back(value);
        return;
    }

    // resolve overflow policy behavior
    match overflow_policy {
        DisplayEventOverflowPolicy::DropNewest => {
            *dropped_count = dropped_count.saturating_add(1);
        }
        DisplayEventOverflowPolicy::DropOldest => {
            let _ = queue.pop_front();
            *dropped_count = dropped_count.saturating_add(1);
            queue.push_back(value);
        }
        DisplayEventOverflowPolicy::Error => {
            *overflow_error_pending = true;
            *dropped_count = dropped_count.saturating_add(1);
        }
    }
}

/// Drain stale weak entries and skip one identity from one weak registry.
pub(super) fn retain_live_without_identity<T>(registry: &mut Vec<Weak<T>>, identity: usize) {
    registry.retain(|weak| {
        let Some(strong) = weak.upgrade() else {
            return false;
        };

        Arc::as_ptr(&strong) as usize != identity
    });
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

/// Query extension support for one connected x11 host.
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

/// Query whether one x11 extension is present.
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
fn query_gamma_control_available(connection: &RustConnection, root: Window) -> RuntimeResult<bool> {
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

    // iterate this sequence
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
        // evaluate this condition
        if gamma_size.size > 0 {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use crate::platform::display::{
        DisplayEventOverflowPolicy, DisplayMonitorEventKindMask, WindowEventKindMask,
    };

    use super::{
        DISPLAY_MONITOR_EVENT_KIND_MASK_ALL, WINDOW_EVENT_KIND_MASK_ALL, monitor_kind_mask,
        push_with_overflow, retain_live_without_identity, validate_monitor_event_kind_mask,
        validate_window_event_kind_mask, window_kind_mask,
    };

    /// Default monitor event-kind masks should include all monitor variants.
    #[test]
    fn test_monitor_kind_mask_defaults_to_all_variants() {
        assert_eq!(monitor_kind_mask(None), DISPLAY_MONITOR_EVENT_KIND_MASK_ALL);
    }

    /// Explicit monitor event-kind masks should preserve caller-selected bits.
    #[test]
    fn test_monitor_kind_mask_preserves_explicit_bits() {
        let expected = 0x15;
        assert_eq!(
            monitor_kind_mask(Some(DisplayMonitorEventKindMask(expected))),
            expected
        );
    }

    /// Default window event-kind masks should include all window variants.
    #[test]
    fn test_window_kind_mask_defaults_to_all_variants() {
        assert_eq!(window_kind_mask(None), WINDOW_EVENT_KIND_MASK_ALL);
    }

    /// Explicit window event-kind masks should preserve caller-selected bits.
    #[test]
    fn test_window_kind_mask_preserves_explicit_bits() {
        let expected = 0x220;
        assert_eq!(
            window_kind_mask(Some(WindowEventKindMask(expected))),
            expected
        );
    }

    /// Unsupported monitor kind-mask bits should be rejected.
    #[test]
    fn test_validate_monitor_event_kind_mask_rejects_unknown_bits() {
        let error = validate_monitor_event_kind_mask(0x8000_0000, "kindMask");
        assert!(error.is_err());
    }

    /// Unsupported window kind-mask bits should be rejected.
    #[test]
    fn test_validate_window_event_kind_mask_rejects_unknown_bits() {
        let error = validate_window_event_kind_mask(0x8000_0000_0000_0000, "kindMask");
        assert!(error.is_err());
    }

    /// Drop-oldest overflow policy should retain the newest bounded sequence.
    #[test]
    fn test_push_with_overflow_drop_oldest_keeps_newest_values() {
        let mut queue = VecDeque::from([1u32, 2u32]);
        let mut overflow_error_pending = false;
        let mut dropped_count = 0u64;

        push_with_overflow(
            &mut queue,
            2,
            DisplayEventOverflowPolicy::DropOldest,
            &mut overflow_error_pending,
            &mut dropped_count,
            3,
        );

        assert_eq!(queue.into_iter().collect::<Vec<_>>(), vec![2, 3]);
        assert!(!overflow_error_pending);
        assert_eq!(dropped_count, 1);
    }

    /// Error overflow policy should preserve queue contents and set overflow pending state.
    #[test]
    fn test_push_with_overflow_error_sets_pending_without_mutating_queue() {
        let mut queue = VecDeque::from([10u32, 20u32]);
        let mut overflow_error_pending = false;
        let mut dropped_count = 0u64;

        push_with_overflow(
            &mut queue,
            2,
            DisplayEventOverflowPolicy::Error,
            &mut overflow_error_pending,
            &mut dropped_count,
            30,
        );

        assert_eq!(queue.into_iter().collect::<Vec<_>>(), vec![10, 20]);
        assert!(overflow_error_pending);
        assert_eq!(dropped_count, 1);
    }

    /// Weak registry compaction should remove stale entries and one requested identity.
    #[test]
    fn test_retain_live_without_identity_removes_stale_and_target_entries() {
        let keep = Arc::new(1u8);
        let skip = Arc::new(2u8);
        let stale = Arc::new(3u8);

        let keep_weak = Arc::downgrade(&keep);
        let skip_weak = Arc::downgrade(&skip);
        let stale_weak = Arc::downgrade(&stale);
        let mut registry = vec![keep_weak, skip_weak, stale_weak];

        let skip_identity = Arc::as_ptr(&skip) as usize;
        drop(stale);
        retain_live_without_identity(&mut registry, skip_identity);

        assert_eq!(registry.len(), 1);
        let kept = registry[0]
            .upgrade()
            .expect("live registry entry should remain after compaction");
        assert_eq!(*kept, 1);
    }
}
