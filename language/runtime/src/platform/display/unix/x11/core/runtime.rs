use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use x11rb::connection::Connection;

use super::connection::X11ConnectionState;
use super::ingress;
use super::xlib::{X11InputContextHandle, destroy_input_context};
use crate::diagnostic::{DiagnosticStore, RuntimeResult};
use crate::host::{HostSessionRegistry, RuntimeIngressHandler};
use crate::platform::display::unix::x11::event::{
    self as x11_event, DisplayEventRecord, MonitorEventStream, WindowEventRecord, WindowEventStream,
};
use crate::platform::display::unix::x11::model::{MonitorSnapshot, X11WindowHostState};
use crate::platform::resource::InputTextSessionHandle;
use crate::platform::{ResourceTable, resource};
use crate::runtime::{
    BindingCallContext, RuntimeEventLog, RuntimeSnapshotCache, RuntimeStreamRegistry,
};

/// Callback-safe reference to the owning worker resource table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct X11ResourceTableRef(*const ResourceTable);

unsafe impl Send for X11ResourceTableRef {}
unsafe impl Sync for X11ResourceTableRef {}

/// Runtime-owned X11 display backend state.
pub(crate) struct X11RuntimeState {
    /// Worker resource table used for callback-owned text-session updates.
    pub(crate) resources: X11ResourceTableRef,
    /// Lazy X11 connection state.
    pub(crate) connection: Mutex<Option<Arc<X11ConnectionState>>>,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
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
    /// Mapping from X11 window id to runtime dispatch payload.
    pub(crate) windows_by_xid: Mutex<HashMap<u32, X11WindowDispatchEntry>>,
    /// Active native text session routing keyed by runtime window handle.
    pub(crate) active_text_sessions: Mutex<HashMap<resource::WindowHandle, InputTextSessionHandle>>,
    /// Active native X11 input contexts keyed by runtime window handle.
    pub(crate) text_input_contexts:
        Mutex<HashMap<resource::WindowHandle, X11TextInputContextState>>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(crate) monitor_topology_snapshot: RuntimeSnapshotCache<Vec<MonitorSnapshot>>,
    /// Registered host-owned ingress observer for this runtime.
    pub(crate) runtime_ingress_handler: OnceLock<Arc<X11RuntimeIngressHandler>>,
    /// One-time service registration guard for this runtime.
    service_registration: OnceLock<()>,
}

/// Runtime dispatch entry for one live X11 window id.
#[derive(Clone)]
pub(crate) struct X11WindowDispatchEntry {
    /// Runtime window handle associated with this X11 window id.
    pub(crate) window: resource::WindowHandle,
    /// Weak handle to the live window host-state payload.
    pub(crate) host_state: Weak<Mutex<X11WindowHostState>>,
}

/// Host-owned ingress observer for one X11 runtime.
#[derive(Debug)]
pub(crate) struct X11RuntimeIngressHandler {
    /// Weak runtime state used for ingress-driven event publication.
    runtime_state: Weak<X11RuntimeState>,
}

/// Native X11 text input context state for one live runtime window.
#[derive(Debug)]
pub(crate) struct X11TextInputContextState {
    /// Native XIM input context handle.
    pub(crate) input_context: X11InputContextHandle,
    /// Callback payload retained for the XIM lifetime.
    pub(crate) callback_payload: *mut X11WindowTextCallbackPayload,
    /// Whether one preedit session is currently active.
    pub(crate) is_composing: bool,
    /// Last published preedit string.
    pub(crate) composition_text: String,
    /// Last published caret offset inside the preedit string.
    pub(crate) composition_caret: i32,
}

/// Callback payload shared with native XIM callbacks.
#[derive(Debug)]
pub(crate) struct X11WindowTextCallbackPayload {
    /// Owning runtime state for the active X11 host.
    pub(crate) runtime_state: Weak<X11RuntimeState>,
    /// Runtime window handle associated with the active XIC.
    pub(crate) window: resource::WindowHandle,
}

unsafe impl Send for X11TextInputContextState {}
unsafe impl Sync for X11TextInputContextState {}

impl Drop for X11TextInputContextState {
    fn drop(&mut self) {
        // release the native xic before freeing the callback payload
        destroy_input_context(self.input_context);

        if !self.callback_payload.is_null() {
            unsafe {
                drop(Box::from_raw(self.callback_payload));
            }
        }
    }
}

impl RuntimeIngressHandler for X11RuntimeIngressHandler {
    /// Service X11 ingress and publish runtime-owned event deltas.
    fn advance_session_ingress(&self) -> RuntimeResult<()> {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return Ok(());
        };

        runtime_state.service_ingress("destack.display")
    }
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
    /// Create one runtime-owned X11 state value.
    pub(crate) fn from_context(binding: &BindingCallContext) -> Self {
        Self {
            resources: X11ResourceTableRef(&binding.worker().resources),
            connection: Mutex::new(None),
            diagnostics: Arc::clone(&binding.worker().diagnostics),
            monitor_events: Mutex::new(RuntimeEventLog::default()),
            monitor_event_signal: Condvar::new(),
            window_events: Mutex::new(RuntimeEventLog::default()),
            window_event_signal: Condvar::new(),
            monitor_streams: RuntimeStreamRegistry::default(),
            window_streams: RuntimeStreamRegistry::default(),
            windows_by_xid: Mutex::new(HashMap::new()),
            active_text_sessions: Mutex::new(HashMap::new()),
            text_input_contexts: Mutex::new(HashMap::new()),
            monitor_topology_snapshot: RuntimeSnapshotCache::default(),
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

    /// Register one live X11 window id mapping.
    pub(crate) fn register_xid(
        &self,
        xid: u32,
        window: resource::WindowHandle,
        host_state: Weak<Mutex<X11WindowHostState>>,
    ) {
        self.windows_by_xid
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(xid, X11WindowDispatchEntry { window, host_state });
    }

    /// Unregister one live X11 window id mapping.
    pub(crate) fn unregister_xid(&self, xid: u32) {
        self.windows_by_xid
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&xid);
    }

    /// Resolve one runtime window dispatch entry from one X11 window id.
    pub(crate) fn window_dispatch_entry(&self, xid: u32) -> Option<X11WindowDispatchEntry> {
        self.windows_by_xid
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get(&xid)
            .cloned()
    }

    /// Service one ingress step for this runtime.
    pub(crate) fn service_ingress(self: &Arc<Self>, operation: &'static str) -> RuntimeResult<()> {
        // resolve host connection state
        let connection_state = super::connection_state(self, operation)?;
        let mut is_monitor_topology_dirty = false;

        // drain pending events until the queue is empty
        loop {
            let event = connection_state
                .connection
                .poll_for_event()
                .map_err(|error| {
                    super::core::io_error(operation, format!("poll_for_event failed: {error}"))
                })?;
            let Some(event) = event else {
                break;
            };

            // record topology-affecting randr events before dispatch consumes them
            is_monitor_topology_dirty |= ingress::event_updates_monitor_topology(&event);

            // dispatch one x11 event into runtime state updates
            ingress::handle_x11_event(self, connection_state.as_ref(), event);
        }

        // publish monitor topology only when randr reported a topology change
        if is_monitor_topology_dirty {
            x11_event::publish_monitor_topology_deltas(self)?;
        }

        Ok(())
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
                Arc::new(X11RuntimeIngressHandler {
                    runtime_state: Arc::downgrade(self),
                })
            })
            .clone();
        let handler: Arc<dyn RuntimeIngressHandler> = observer;

        HostSessionRegistry::register_session_ingress_handler(host_session_id, &handler)
    }

    /// Register this runtime with the x11 display service once.
    pub(crate) fn ensure_service_registration(self: &Arc<Self>, context: &BindingCallContext) {
        // register once so repeated binding calls do not keep re-entering the ingress setup path
        self.service_registration.get_or_init(|| {
            let service = context.worker().platform_state.display.x11_service();
            service.register_runtime(context, self);
        });
    }
}

/// Return runtime-owned X11 state for this binding call.
pub(crate) fn runtime_state(binding: &BindingCallContext) -> Arc<X11RuntimeState> {
    let runtime_state = binding
        .worker()
        .platform_state
        .display
        .x11_runtime_state(|| X11RuntimeState::from_context(binding));

    runtime_state.ensure_service_registration(binding);
    runtime_state
}
