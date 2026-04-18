use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use super::WaylandConnectionState;
use crate::diagnostic::RuntimeResult;
use crate::host::{HostSessionRegistry, RuntimeIngressHandler};
use crate::platform::display::DisplayBackend;
use crate::platform::display::unix::wayland::event::{
    self as wayland_event, DisplayEventRecord, MonitorEventStream, WindowEventRecord,
    WindowEventStream,
};
use crate::platform::display::unix::wayland::model::{MonitorSnapshot, WaylandWindowHostState};
use crate::platform::resource::InputTextSessionHandle;
use crate::platform::{ResourceTable, resource};
use crate::runtime::{
    BindingCallContext, RuntimeEventLog, RuntimeSnapshotCache, RuntimeStreamRegistry,
};

/// Callback-safe reference to the owning worker resource table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WaylandResourceTableRef(*const ResourceTable);

unsafe impl Send for WaylandResourceTableRef {}
unsafe impl Sync for WaylandResourceTableRef {}

/// Cached gamma-ramp payload for one display.
#[derive(Debug, Clone, Default)]
pub(crate) struct WaylandGammaRampSnapshot {
    /// Cached red channel values.
    pub(crate) red: Vec<u16>,
    /// Cached green channel values.
    pub(crate) green: Vec<u16>,
    /// Cached blue channel values.
    pub(crate) blue: Vec<u16>,
}

/// Event dispatch token for one wayland window object.
#[derive(Debug, Clone)]
pub(crate) struct WaylandWindowDispatchToken {
    /// Stable runtime window identifier.
    pub(crate) window_id: String,
    /// Weak handle to the runtime window host-state payload.
    pub(crate) host_state: Weak<Mutex<WaylandWindowHostState>>,
}

/// Runtime-owned Wayland display backend state.
pub(crate) struct WaylandRuntimeState {
    /// Worker resource table used for callback-owned text-session updates.
    pub(crate) resources: WaylandResourceTableRef,
    /// Lazy wayland host connection state.
    pub(crate) connection_state: Mutex<Option<Arc<WaylandConnectionState>>>,
    /// Runtime-owned monitor-event log.
    pub(crate) monitor_events: Mutex<RuntimeEventLog<DisplayEventRecord>>,
    /// Wake signal for monitor-event readers.
    pub(crate) monitor_event_signal: Condvar,
    /// Runtime-owned window-event log.
    pub(crate) window_events: Mutex<RuntimeEventLog<WindowEventRecord>>,
    /// Wake signal for window-event readers.
    pub(crate) window_event_signal: Condvar,
    /// Registered monitor-event streams for this runtime.
    pub(crate) monitor_streams: RuntimeStreamRegistry<MonitorEventStream>,
    /// Registered window-event streams for this runtime.
    pub(crate) window_streams: RuntimeStreamRegistry<WindowEventStream>,
    /// Mapping from runtime window id to runtime window handle.
    pub(crate) windows_by_id: Mutex<HashMap<String, resource::WindowHandle>>,
    /// Mapping from wayland surface object id to runtime window dispatch token.
    pub(crate) window_tokens_by_surface:
        Mutex<HashMap<wayland_client::backend::ObjectId, WaylandWindowDispatchToken>>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(crate) monitor_topology_snapshot: RuntimeSnapshotCache<Vec<MonitorSnapshot>>,
    /// Cached gamma-ramp payloads keyed by stable display id.
    pub(crate) gamma_ramps_by_display_id: Mutex<HashMap<String, WaylandGammaRampSnapshot>>,
    /// Active native text session routing keyed by runtime window handle.
    pub(crate) active_text_sessions: Mutex<HashMap<resource::WindowHandle, InputTextSessionHandle>>,
    /// Monotonic id generator for backend-local host window identifiers.
    next_window_host_id: AtomicU64,
    /// Registered host-owned ingress observer for this runtime.
    runtime_ingress_handler: OnceLock<Arc<WaylandRuntimeIngressHandler>>,
    /// One-time service registration guard for this runtime.
    service_registration: OnceLock<()>,
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
    /// Create one runtime-owned Wayland state value.
    pub(crate) fn from_context(_context: &BindingCallContext) -> Self {
        Self {
            resources: WaylandResourceTableRef(&_context.worker().resources),
            connection_state: Mutex::new(None),
            monitor_events: Mutex::new(RuntimeEventLog::default()),
            monitor_event_signal: Condvar::new(),
            window_events: Mutex::new(RuntimeEventLog::default()),
            window_event_signal: Condvar::new(),
            monitor_streams: RuntimeStreamRegistry::default(),
            window_streams: RuntimeStreamRegistry::default(),
            windows_by_id: Mutex::new(HashMap::new()),
            window_tokens_by_surface: Mutex::new(HashMap::new()),
            monitor_topology_snapshot: RuntimeSnapshotCache::default(),
            gamma_ramps_by_display_id: Mutex::new(HashMap::new()),
            active_text_sessions: Mutex::new(HashMap::new()),
            next_window_host_id: AtomicU64::new(1),
            runtime_ingress_handler: OnceLock::new(),
            service_registration: OnceLock::new(),
        }
    }

    /// Borrow the worker resource table captured by this runtime.
    pub(crate) fn resource_table(&self) -> &ResourceTable {
        unsafe { &*self.resources.0 }
    }

    /// Allocate one stable monitor-event stream identifier.
    pub(crate) fn next_monitor_stream_id(&self) -> u64 {
        self.monitor_streams.next_stream_id()
    }

    /// Allocate one stable window-event stream identifier.
    pub(crate) fn next_window_stream_id(&self) -> u64 {
        self.window_streams.next_stream_id()
    }

    /// Allocate one stable host-side wayland window identifier.
    pub(crate) fn next_window_host_id(&self) -> u64 {
        self.next_window_host_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Register one monitor-event stream for this runtime.
    pub(crate) fn register_monitor_stream(&self, stream: Arc<MonitorEventStream>) {
        self.monitor_streams.register(stream.stream_id, stream);
    }

    /// Unregister one monitor-event stream for this runtime.
    pub(crate) fn unregister_monitor_stream(
        &self,
        stream_id: u64,
    ) -> Option<Arc<MonitorEventStream>> {
        self.monitor_streams.unregister(stream_id)
    }

    /// Return one snapshot of the live monitor-event streams.
    pub(crate) fn monitor_streams_snapshot(&self) -> Vec<Arc<MonitorEventStream>> {
        self.monitor_streams.snapshot()
    }

    /// Register one window-event stream for this runtime.
    pub(crate) fn register_window_stream(&self, stream: Arc<WindowEventStream>) {
        self.window_streams.register(stream.stream_id, stream);
    }

    /// Unregister one window-event stream for this runtime.
    pub(crate) fn unregister_window_stream(
        &self,
        stream_id: u64,
    ) -> Option<Arc<WindowEventStream>> {
        self.window_streams.unregister(stream_id)
    }

    /// Return one snapshot of the live window-event streams.
    pub(crate) fn window_streams_snapshot(&self) -> Vec<Arc<WindowEventStream>> {
        self.window_streams.snapshot()
    }

    /// Initialize the cached monitor topology snapshot when no baseline exists yet.
    pub(crate) fn initialize_monitor_topology_snapshot(
        &self,
        snapshots: Vec<MonitorSnapshot>,
    ) -> bool {
        self.monitor_topology_snapshot.initialize(snapshots)
    }

    /// Replace the cached monitor topology snapshot with one fresh baseline.
    pub(crate) fn reset_monitor_topology_snapshot(&self, snapshots: Vec<MonitorSnapshot>) {
        self.monitor_topology_snapshot.reset(snapshots);
    }

    /// Replace the cached monitor topology snapshot and return the observed delta records.
    pub(crate) fn replace_monitor_topology_snapshot(
        &self,
        snapshots: Vec<MonitorSnapshot>,
        build_records: impl FnOnce(&[MonitorSnapshot], &[MonitorSnapshot]) -> Vec<DisplayEventRecord>,
    ) -> Option<Vec<DisplayEventRecord>> {
        self.monitor_topology_snapshot.replace(
            snapshots,
            |previous_snapshots, current_snapshots| {
                build_records(previous_snapshots, current_snapshots)
            },
        )
    }

    /// Service one ingress step for this runtime.
    pub(crate) fn service_ingress(self: &Arc<Self>, operation: &'static str) -> RuntimeResult<()> {
        // dispatch pending wayland protocol events first
        let is_monitor_topology_dirty =
            super::connection::dispatch_pending_for_runtime(self, operation)?;

        // publish monitor topology only when protocol dispatch mutated it
        if is_monitor_topology_dirty {
            wayland_event::publish_monitor_topology_deltas(self)?;
        }

        Ok(())
    }

    /// Resolve one runtime window handle from one stable wayland window id.
    pub(crate) fn window_handle_from_id(&self, window_id: &str) -> Option<resource::WindowHandle> {
        self.windows_by_id
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(window_id)
            .copied()
    }

    /// Register one runtime window handle for one stable wayland window id.
    pub(crate) fn register_window_handle(&self, window_id: String, window: resource::WindowHandle) {
        self.windows_by_id
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(window_id, window);
    }

    /// Unregister one runtime window handle for one stable wayland window id.
    pub(crate) fn unregister_window_handle(&self, window_id: &str) {
        self.windows_by_id
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(window_id);
    }

    /// Resolve one runtime window dispatch token from one wayland surface id.
    pub(crate) fn window_token_from_surface(
        &self,
        surface_id: &wayland_client::backend::ObjectId,
    ) -> Option<WaylandWindowDispatchToken> {
        self.window_tokens_by_surface
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(surface_id)
            .cloned()
    }

    /// Register one runtime window dispatch token for one surface id.
    pub(crate) fn register_window_token(
        &self,
        surface_id: wayland_client::backend::ObjectId,
        token: WaylandWindowDispatchToken,
    ) {
        self.window_tokens_by_surface
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(surface_id, token);
    }

    /// Unregister one runtime window dispatch token for one surface id.
    pub(crate) fn unregister_window_token(&self, surface_id: &wayland_client::backend::ObjectId) {
        self.window_tokens_by_surface
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(surface_id);
    }

    /// Register one host-owned ingress observer for this runtime.
    pub(crate) fn register_runtime_ingress(
        self: &Arc<Self>,
        context: &BindingCallContext,
    ) -> RuntimeResult<()> {
        let host_session_id = context.host().host_session_id();

        let observer = self
            .runtime_ingress_handler
            .get_or_init(|| {
                Arc::new(WaylandRuntimeIngressHandler {
                    runtime_state: Arc::downgrade(self),
                })
            })
            .clone();
        let handler: Arc<dyn RuntimeIngressHandler> = observer;

        HostSessionRegistry::register_session_ingress_handler(host_session_id, &handler)
    }

    /// Register this runtime with the wayland display service once.
    fn ensure_service_registration(self: &Arc<Self>, context: &BindingCallContext) {
        // register once so repeated binding calls do not keep re-entering the ingress setup path
        self.service_registration.get_or_init(|| {
            let service = context.worker().platform_state.display.wayland_service();
            service.register_runtime(context, self);
        });
    }
}

/// Host-owned ingress observer for one wayland runtime.
#[derive(Debug)]
struct WaylandRuntimeIngressHandler {
    /// Weak runtime state used for ingress-driven event publication.
    runtime_state: Weak<WaylandRuntimeState>,
}

impl RuntimeIngressHandler for WaylandRuntimeIngressHandler {
    /// Service wayland ingress and publish runtime-owned event deltas.
    fn advance_session_ingress(&self) -> RuntimeResult<()> {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return Ok(());
        };

        runtime_state.service_ingress("destack.display")
    }
}

/// Return the backend for the wayland backend implementation.
pub(crate) fn selected_backend() -> DisplayBackend {
    DisplayBackend::Wayland
}

/// Return runtime-owned Wayland state for this binding call.
pub(crate) fn runtime_state(context: &BindingCallContext) -> Arc<WaylandRuntimeState> {
    let runtime_state = context
        .worker()
        .platform_state
        .display
        .wayland_runtime_state(|| WaylandRuntimeState::from_context(context));

    runtime_state.ensure_service_registration(context);
    runtime_state
}
