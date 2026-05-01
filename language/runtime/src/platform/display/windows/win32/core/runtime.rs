use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use windows_sys::Win32::Foundation::{GetLastError, HWND, SetLastError};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWLP_USERDATA, GetWindowLongPtrW, IsWindow, SetWindowLongPtrW,
};

use crate::diagnostic::{DiagnosticStore, RuntimeResult};
use crate::host::{HostSessionId, HostSessionRegistry, RuntimeIngressHandler};
use crate::platform::display::WindowCursorMode;
use crate::platform::display::windows::win32::event::{
    DisplayEventRecord, MonitorEventStream, WindowEventRecord, WindowEventStream,
};
use crate::platform::display::windows::win32::model::{MonitorSnapshot, Win32WindowHostState};
use crate::platform::display::windows::win32::{core, publish_monitor_topology_deltas};
use crate::platform::resource::{self, InputTextSessionHandle, ResourceTable};
use crate::runtime::{
    BindingCallContext, RuntimeEventLog, RuntimeSnapshotCache, RuntimeStreamRegistry,
};

/// Callback-safe reference to the owning worker resource table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Win32ResourceTableRef(*const ResourceTable);

unsafe impl Send for Win32ResourceTableRef {}
unsafe impl Sync for Win32ResourceTableRef {}

/// Runtime-owned mutable state for the Win32 display backend.
#[derive(Debug)]
pub(crate) struct Win32RuntimeState {
    /// Worker resource table used for callback-owned input-session updates.
    pub(crate) resources: Win32ResourceTableRef,
    /// Shared global cursor visibility state.
    pub(crate) cursor_visible_state: Mutex<Option<bool>>,
    /// Per-window cursor policy lanes used to derive process-global cursor state.
    pub(crate) cursor_policy_by_window: Mutex<HashMap<resource::WindowHandle, CursorPolicyState>>,
    /// Monotonic counter used for stable cursor policy ordering.
    pub(crate) next_cursor_policy_sequence: AtomicU64,
    /// Monotonic counter used for stable runtime window identifiers.
    next_window_identifier: AtomicU64,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// Runtime-owned monitor-event log.
    pub(crate) monitor_events: Mutex<RuntimeEventLog<DisplayEventRecord>>,
    /// Wake signal for monitor-event readers.
    pub(crate) monitor_event_signal: Condvar,
    /// Registered monitor-event streams for this runtime.
    pub(crate) monitor_streams: RuntimeStreamRegistry<MonitorEventStream>,
    /// Last observed monitor topology snapshot.
    pub(crate) monitor_topology_snapshot: RuntimeSnapshotCache<Vec<MonitorSnapshot>>,
    /// Runtime-owned window-event log.
    pub(crate) window_events: Mutex<RuntimeEventLog<WindowEventRecord>>,
    /// Wake signal for window-event readers.
    pub(crate) window_event_signal: Condvar,
    /// Registered window-event streams for this runtime.
    pub(crate) window_streams: RuntimeStreamRegistry<WindowEventStream>,
    /// Active native text session routing keyed by runtime window handle.
    pub(crate) active_text_sessions: Mutex<HashMap<resource::WindowHandle, InputTextSessionHandle>>,
    /// Registered host-owned ingress observer for this runtime.
    runtime_ingress_handler: OnceLock<Arc<Win32RuntimeIngressHandler>>,
    /// One-time Win32 service registration guard for this runtime.
    service_registration: OnceLock<()>,
}

impl Default for Win32RuntimeState {
    /// Create one default Win32 runtime state.
    fn default() -> Self {
        Self::new(
            Arc::new(DiagnosticStore::default()),
            Win32ResourceTableRef(std::ptr::null()),
        )
    }
}

impl Win32RuntimeState {
    /// Create one Win32 runtime state with explicit diagnostics storage.
    fn new(diagnostics: Arc<DiagnosticStore>, resources: Win32ResourceTableRef) -> Self {
        Self {
            resources,
            cursor_visible_state: Mutex::new(None),
            cursor_policy_by_window: Mutex::new(HashMap::new()),
            next_cursor_policy_sequence: AtomicU64::new(1),
            next_window_identifier: AtomicU64::new(1),
            diagnostics,
            monitor_events: Mutex::new(RuntimeEventLog::default()),
            monitor_event_signal: Condvar::new(),
            monitor_streams: RuntimeStreamRegistry::default(),
            monitor_topology_snapshot: RuntimeSnapshotCache::default(),
            window_events: Mutex::new(RuntimeEventLog::default()),
            window_event_signal: Condvar::new(),
            window_streams: RuntimeStreamRegistry::default(),
            active_text_sessions: Mutex::new(HashMap::new()),
            runtime_ingress_handler: OnceLock::new(),
            service_registration: OnceLock::new(),
        }
    }

    /// Allocate one stable runtime window identifier.
    pub(crate) fn next_window_identifier(&self) -> u64 {
        self.next_window_identifier.fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate one stable cursor-policy sequence number.
    pub(crate) fn next_cursor_policy_sequence(&self) -> u64 {
        self.next_cursor_policy_sequence
            .fetch_add(1, Ordering::Relaxed)
    }

    /// Allocate one stable monitor-event stream identifier.
    pub(crate) fn next_monitor_stream_id(&self) -> u64 {
        self.monitor_streams.next_stream_id()
    }

    /// Allocate one stable window-event stream identifier.
    pub(crate) fn next_window_stream_id(&self) -> u64 {
        self.window_streams.next_stream_id()
    }

    /// Borrow the worker resource table captured by this runtime.
    pub(crate) fn resource_table(&self) -> &ResourceTable {
        // safety: the worker owns the resource table for the lifetime of the runtime state
        unsafe { &*self.resources.0 }
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
}

/// Runtime mapping payload for one live hwnd.
#[derive(Clone)]
pub(crate) struct Win32WindowDispatchEntry {
    /// Runtime window handle associated with this hwnd.
    pub(crate) window: resource::WindowHandle,
    /// Weak host-state reference for this window.
    pub(crate) host_state: Weak<Mutex<Win32WindowHostState>>,
    /// Runtime-owned Win32 backend state.
    pub(crate) runtime_state: Arc<Win32RuntimeState>,
}

impl Win32WindowDispatchEntry {
    /// Register one live hwnd mapping for runtime window callbacks.
    pub(crate) fn register(hwnd: HWND, entry: Self, operation: &'static str) -> RuntimeResult<()> {
        let entry = Box::new(entry);
        let entry = Box::into_raw(entry);
        let previous = unsafe {
            SetLastError(0);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, entry as isize)
        };

        if previous == 0 {
            let error_code = unsafe { GetLastError() };

            if error_code != 0 {
                unsafe {
                    drop(Box::from_raw(entry));
                }
                return Err(core::io_error_with_code(
                    operation,
                    "SetWindowLongPtrW",
                    error_code as u32,
                    "failed to register runtime window entry",
                ));
            }
        }

        Ok(())
    }

    /// Unregister one live hwnd mapping.
    pub(crate) fn unregister(hwnd: HWND) {
        let pointer =
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut Win32WindowDispatchEntry;

        if pointer.is_null() {
            return;
        }

        unsafe {
            drop(Box::from_raw(pointer));
        }
    }

    /// Resolve one runtime hwnd entry.
    pub(crate) fn from_hwnd(hwnd: HWND) -> Option<Self> {
        let pointer =
            unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut Win32WindowDispatchEntry;

        if pointer.is_null() {
            return None;
        }

        let entry = unsafe { &*pointer };
        Some(entry.clone())
    }
}

/// Resolve whether one live window handle is still valid.
pub(crate) fn is_live_hwnd(hwnd: HWND) -> bool {
    unsafe { IsWindow(hwnd) != 0 }
}

/// Per-window cursor policy snapshot.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CursorPolicyState {
    /// Host window handle associated with this policy.
    pub(crate) hwnd: windows_sys::Win32::Foundation::HWND,
    /// Per-window cursor visibility preference.
    pub(crate) cursor_visible: bool,
    /// Per-window cursor mode preference.
    pub(crate) cursor_mode: WindowCursorMode,
    /// Monotonic sequence used for most-recent policy ordering.
    pub(crate) sequence: u64,
}

/// Host-owned ingress observer for one Win32 display runtime.
#[derive(Debug)]
struct Win32RuntimeIngressHandler {
    /// Weak runtime state used for monitor-topology publication.
    runtime_state: Weak<Win32RuntimeState>,
}

impl RuntimeIngressHandler for Win32RuntimeIngressHandler {
    /// Publish monitor-topology deltas after one host ingress service step.
    fn advance_session_ingress(&self) -> RuntimeResult<()> {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return Ok(());
        };

        runtime_state.publish_monitor_topology_deltas()
    }
}

impl Win32RuntimeState {
    /// Publish monitor-topology deltas for this runtime.
    pub(crate) fn publish_monitor_topology_deltas(self: &Arc<Self>) -> RuntimeResult<()> {
        publish_monitor_topology_deltas(self)
    }

    /// Register one host-owned ingress observer for this runtime.
    pub(crate) fn register_runtime_ingress(
        self: &Arc<Self>,
        host_session_id: HostSessionId,
    ) -> RuntimeResult<()> {
        let observer = self
            .runtime_ingress_handler
            .get_or_init(|| {
                Arc::new(Win32RuntimeIngressHandler {
                    runtime_state: Arc::downgrade(self),
                })
            })
            .clone();
        let handler: Arc<dyn RuntimeIngressHandler> = observer;

        HostSessionRegistry::register_session_ingress_handler(host_session_id, &handler)
    }

    /// Register this runtime with the Win32 display service once.
    pub(crate) fn ensure_service_registration(self: &Arc<Self>, context: &BindingCallContext) {
        // register once so repeated binding calls do not keep re-entering the windows loop
        self.service_registration.get_or_init(|| {
            let service = context.worker().platform_state.display.win32_service();
            service.register_runtime(context, self);
        });
    }
}

/// Return runtime-owned Win32 display state.
pub(crate) fn runtime_state(context: &BindingCallContext) -> Arc<Win32RuntimeState> {
    let diagnostics = Arc::clone(&context.worker().diagnostics);
    let runtime_state = context
        .worker()
        .platform_state
        .display
        .win32_runtime_state(|| {
            Win32RuntimeState::new(
                diagnostics,
                Win32ResourceTableRef(&context.worker().resources),
            )
        });

    // keep the host message loop registration service-backed
    runtime_state.ensure_service_registration(context);
    runtime_state
}
