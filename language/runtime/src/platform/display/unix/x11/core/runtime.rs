use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use x11rb::connection::Connection;

use super::connection::X11ConnectionState;
use super::ingress;
use crate::diagnostic::AgentDiagnosticStore;
use crate::host::{RuntimeIngressObserver, register_runtime_ingress_observer};
use crate::platform::display::unix::x11::event::{
    self as x11_event, DisplayEventRecord, MonitorEventStream, WindowEventRecord, WindowEventStream,
};
use crate::platform::display::unix::x11::model::{MonitorSnapshot, X11WindowHostState};
use crate::platform::display::{RuntimeEventLog, RuntimeSnapshotCache, RuntimeStreamRegistry};
use crate::platform::resource;
use crate::runtime::BindingCallContext;

/// Runtime-owned X11 display backend state.
pub(crate) struct X11RuntimeState {
    /// Lazy X11 connection state.
    pub(crate) connection: Mutex<Option<Arc<X11ConnectionState>>>,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(crate) diagnostics: Arc<AgentDiagnosticStore>,
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
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(crate) monitor_topology_snapshot: RuntimeSnapshotCache<Vec<MonitorSnapshot>>,
    /// Registered host-owned ingress observer for this runtime.
    pub(crate) runtime_ingress_observer: OnceLock<Arc<X11RuntimeIngressObserver>>,
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
pub(crate) struct X11RuntimeIngressObserver {
    /// Weak runtime state used for ingress-driven event publication.
    runtime_state: Weak<X11RuntimeState>,
}

impl RuntimeIngressObserver for X11RuntimeIngressObserver {
    /// Service X11 ingress and publish runtime-owned event deltas.
    fn process_runtime_ingress(&self) -> crate::diagnostic::RuntimeResult<()> {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return Ok(());
        };

        runtime_state.process_runtime_ingress("destack.display")
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
            connection: Mutex::new(None),
            diagnostics: Arc::clone(&binding.agent().diagnostic),
            monitor_events: Mutex::new(RuntimeEventLog::default()),
            monitor_event_signal: Condvar::new(),
            window_events: Mutex::new(RuntimeEventLog::default()),
            window_event_signal: Condvar::new(),
            monitor_streams: RuntimeStreamRegistry::default(),
            window_streams: RuntimeStreamRegistry::default(),
            windows_by_xid: Mutex::new(HashMap::new()),
            monitor_topology_snapshot: RuntimeSnapshotCache::default(),
            runtime_ingress_observer: OnceLock::new(),
        }
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
    pub(crate) fn process_runtime_ingress(
        self: &Arc<Self>,
        operation: &'static str,
    ) -> crate::diagnostic::RuntimeResult<()> {
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
    pub(crate) fn register_runtime_ingress(self: &Arc<Self>, context: &BindingCallContext) {
        let runtime_id = context.agent().runtime_id;

        let observer = self
            .runtime_ingress_observer
            .get_or_init(|| {
                Arc::new(X11RuntimeIngressObserver {
                    runtime_state: Arc::downgrade(self),
                })
            })
            .clone();
        let observer: Arc<dyn RuntimeIngressObserver> = observer;

        register_runtime_ingress_observer(runtime_id.0, &observer);
    }
}

/// Return runtime-owned X11 state for this binding call.
pub(crate) fn runtime_state(binding: &BindingCallContext) -> Arc<X11RuntimeState> {
    let runtime_state = binding
        .agent()
        .platform_state
        .display
        .x11_runtime_state(|| X11RuntimeState::from_context(binding));

    runtime_state.register_runtime_ingress(binding);
    runtime_state
}
