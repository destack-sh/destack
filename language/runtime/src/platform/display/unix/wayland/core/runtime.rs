use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::DisplayBackend;
use crate::platform::resource;
use crate::runtime::BindingCallContext;

use super::super::event::{MonitorEventBinding, WindowEventBinding};
use super::super::model::{MonitorSnapshot, WaylandGammaRampSnapshot, WaylandWindowDispatchToken};
use super::{WaylandConnectionState, WaylandRuntimeState};

/// Runtime-owned wayland display backend state.
pub(crate) struct WaylandRuntimeState {
    /// Lazy wayland host connection state.
    pub(super) connection_state: Mutex<Option<Arc<WaylandConnectionState>>>,
    /// Monitor-event subscribers for this runtime.
    pub(super) monitor_event_registry: Mutex<Vec<Weak<MonitorEventBinding>>>,
    /// Window-event subscribers for this runtime.
    pub(super) window_event_registry: Mutex<Vec<Weak<WindowEventBinding>>>,
    /// Mapping from runtime window id to runtime window handle.
    pub(super) windows_by_id: Mutex<HashMap<String, resource::WindowHandle>>,
    /// Mapping from wayland surface object id to runtime window dispatch token.
    pub(super) window_tokens_by_surface:
        Mutex<HashMap<wayland_client::backend::ObjectId, WaylandWindowDispatchToken>>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(super) monitor_topology_snapshot: Mutex<Option<Vec<MonitorSnapshot>>>,
    /// Cached gamma-ramp payloads keyed by stable display id.
    pub(super) gamma_ramps_by_display_id: Mutex<HashMap<String, WaylandGammaRampSnapshot>>,
    /// Monotonic id generator for backend-local host window identifiers.
    next_window_host_id: AtomicU64,
}

impl std::fmt::Debug for WaylandRuntimeState {
    /// Format this runtime state for diagnostics.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WaylandRuntimeState")
            .finish_non_exhaustive()
    }
}

impl WaylandRuntimeState {
    /// Create one runtime-owned wayland state value.
    pub(super) fn from_context(_context: &BindingCallContext) -> Self {
        Self {
            connection_state: Mutex::new(None),
            monitor_event_registry: Mutex::new(Vec::new()),
            window_event_registry: Mutex::new(Vec::new()),
            windows_by_id: Mutex::new(HashMap::new()),
            window_tokens_by_surface: Mutex::new(HashMap::new()),
            monitor_topology_snapshot: Mutex::new(None),
            gamma_ramps_by_display_id: Mutex::new(HashMap::new()),
            next_window_host_id: AtomicU64::new(1),
        }
    }
}

/// Return the backend for the wayland backend implementation.
pub(crate) fn selected_backend() -> DisplayBackend {
    DisplayBackend::Wayland
}

/// Return runtime-owned wayland state for this binding call.
pub(super) fn runtime_state(context: &BindingCallContext) -> Arc<WaylandRuntimeState> {
    context
        .runtime()
        .platform_state
        .display
        .wayland_runtime_state(|| WaylandRuntimeState::from_context(context))
}

/// Resolve one runtime window handle from one stable wayland window id.
pub(super) fn window_handle_from_id(
    runtime_state: &Arc<WaylandRuntimeState>,
    window_id: &str,
) -> Option<resource::WindowHandle> {
    runtime_state
        .windows_by_id
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(window_id)
        .copied()
}

/// Resolve one runtime window dispatch token from one wayland surface id.
pub(super) fn window_token_from_surface(
    runtime_state: &Arc<WaylandRuntimeState>,
    surface_id: &wayland_client::backend::ObjectId,
) -> Option<WaylandWindowDispatchToken> {
    runtime_state
        .window_tokens_by_surface
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .get(surface_id)
        .cloned()
}

/// Allocate one stable host-side wayland window identifier.
pub(super) fn next_window_host_id(context: &BindingCallContext) -> u64 {
    let runtime_state = runtime_state(context);
    runtime_state
        .next_window_host_id
        .fetch_add(1, Ordering::Relaxed)
}
