use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use dispatch2::{MainThreadBound, run_on_main};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSImage, NSWindow};
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayChangeSummaryFlags, CGDisplayReconfigurationCallBack,
    CGDisplayRegisterReconfigurationCallback, CGError,
};

use super::delegate::AppKitWindowDelegate;
use crate::diagnostic::{DiagnosticStore, RuntimeResult};
use crate::host::core::observer::{RuntimeIngressObserver, register_runtime_ingress_observer};
use crate::platform::display::unix::appkit::event::{
    self as appkit_event, DisplayEventRecord, MonitorEventStream, WindowEventRecord,
    WindowEventStream,
};
use crate::platform::display::unix::appkit::model::{AppKitWindowHostState, MonitorSnapshot};
use crate::platform::display::unix::appkit::window;
use crate::platform::display::{WindowPosition, WindowTheme};
use crate::platform::{ResourceTable, core as core_platform, resource};
use crate::runtime::world::World;
use crate::runtime::{
    BindingCallContext, RuntimeEventLog, RuntimeSnapshotCache, RuntimeStreamRegistry,
};

/// Transient drag and drop state for one native AppKit window.
#[derive(Debug, Default)]
pub(crate) struct AppKitDropSessionState {
    /// Whether one drag session has started for this window.
    pub(crate) started: bool,
    /// Whether one drop operation completed successfully for this session.
    pub(crate) completed: bool,
    /// The last hovered file path when one file payload is active.
    pub(crate) last_hovered_path: Option<String>,
    /// The last known drag position in window coordinates.
    pub(crate) position: Option<WindowPosition>,
}

/// Main-thread AppKit host payload for one opened runtime window.
pub(crate) struct AppKitWindowHost {
    /// The runtime host-state payload for this window.
    pub(crate) host_state: Arc<Mutex<AppKitWindowHostState>>,
    /// The native AppKit window object.
    pub(crate) window: objc2::rc::Retained<NSWindow>,
    /// The native AppKit delegate object.
    pub(crate) _delegate: objc2::rc::Retained<AppKitWindowDelegate>,
    /// Transient drag and drop state for this host window.
    pub(crate) drop_session: RefCell<AppKitDropSessionState>,
    /// Retained miniwindow icon image for this host window.
    pub(crate) window_icon: RefCell<Option<objc2::rc::Retained<NSImage>>>,
}

/// Main-thread AppKit backend state.
pub(crate) struct AppKitMainThreadState {
    /// Mapping from runtime window handle to native host payload.
    pub(crate) windows: HashMap<resource::WindowHandle, AppKitWindowHost>,
}

/// Callback-safe reference to the owning agent resource table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AppKitResourceTableRef(*const ResourceTable);

unsafe impl Send for AppKitResourceTableRef {}
unsafe impl Sync for AppKitResourceTableRef {}

/// Callback-safe reference to the owning runtime world.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AppKitWorldRef(*const World);

unsafe impl Send for AppKitWorldRef {}
unsafe impl Sync for AppKitWorldRef {}

/// Host-owned thread-message observer for one AppKit runtime.
#[derive(Debug)]
struct AppKitIngressObserver {
    /// Weak runtime state used for post-pump reconciliation.
    runtime_state: Weak<AppKitRuntimeState>,
}

impl RuntimeIngressObserver for AppKitIngressObserver {
    /// Service AppKit ingress after one host message pump step.
    fn process_runtime_ingress(&self) -> RuntimeResult<()> {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return Ok(());
        };

        let theme = window::current_window_theme();
        let mut current_theme = runtime_state
            .current_theme
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        // skip global reconciliation when the application theme is unchanged
        if *current_theme == theme {
            return Ok(());
        }

        *current_theme = theme;
        drop(current_theme);

        // keep application theme changes host authoritative without rescanning all window state
        window::reconcile_all_host_window_themes(&runtime_state, theme);
        Ok(())
    }
}

/// Runtime-owned AppKit display backend state.
pub(crate) struct AppKitRuntimeState {
    /// Main-thread state for native AppKit objects.
    pub(crate) main_thread_state: MainThreadBound<RefCell<AppKitMainThreadState>>,
    /// Agent resource table used for callback-owned display-handle updates.
    pub(crate) resources: AppKitResourceTableRef,
    /// Runtime world used for callback-owned resource-table updates.
    pub(crate) world: AppKitWorldRef,
    /// Cached display handles keyed by stable AppKit display identifier.
    pub(crate) display_handle_cache: Mutex<HashMap<String, resource::DisplayHandle>>,
    /// Runtime-owned monitor-event log.
    pub(crate) monitor_events: Mutex<RuntimeEventLog<DisplayEventRecord>>,
    /// Wake signal for monitor-event readers.
    pub(crate) monitor_event_signal: Condvar,
    /// Runtime-owned window-event log.
    pub(crate) window_events: Mutex<RuntimeEventLog<WindowEventRecord>>,
    /// Wake signal for window-event readers.
    pub(crate) window_event_signal: Condvar,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// The last application theme observed from the host.
    pub(crate) current_theme: Mutex<WindowTheme>,
    /// Registered monitor-event streams for this runtime.
    pub(crate) monitor_streams: RuntimeStreamRegistry<MonitorEventStream>,
    /// Registered window-event streams for this runtime.
    pub(crate) window_streams: RuntimeStreamRegistry<WindowEventStream>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(crate) monitor_topology_snapshot: RuntimeSnapshotCache<Vec<MonitorSnapshot>>,
    /// Whether the process-global AppKit cursor is currently hidden.
    pub(crate) cursor_hidden: Mutex<bool>,
    /// Registered host-owned observer for post-pump AppKit reconciliation.
    runtime_ingress_observer: OnceLock<Arc<AppKitIngressObserver>>,
}

/// Shared runtime registry for process-global CoreGraphics display callbacks.
static APPKIT_MONITOR_CALLBACK_RUNTIMES: OnceLock<Mutex<HashMap<u64, Weak<AppKitRuntimeState>>>> =
    OnceLock::new();

/// Guard that ensures the CoreGraphics display callback is registered once.
static APPKIT_MONITOR_CALLBACK_REGISTRATION: OnceLock<()> = OnceLock::new();

impl std::fmt::Debug for AppKitRuntimeState {
    /// Format this runtime state for diagnostics.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppKitRuntimeState")
            .finish_non_exhaustive()
    }
}

impl AppKitRuntimeState {
    /// Create one runtime-owned AppKit state value.
    pub(crate) fn from_context(binding: &BindingCallContext) -> Self {
        let main_thread_state = run_on_main(|mtm| {
            let application = NSApplication::sharedApplication(mtm);
            application.setActivationPolicy(NSApplicationActivationPolicy::Regular);

            MainThreadBound::new(
                RefCell::new(AppKitMainThreadState {
                    windows: HashMap::new(),
                }),
                mtm,
            )
        });

        Self {
            main_thread_state,
            resources: AppKitResourceTableRef(&binding.agent().resources),
            world: AppKitWorldRef(binding.world()),
            display_handle_cache: Mutex::new(HashMap::new()),
            monitor_events: Mutex::new(RuntimeEventLog::default()),
            monitor_event_signal: Condvar::new(),
            window_events: Mutex::new(RuntimeEventLog::default()),
            window_event_signal: Condvar::new(),
            diagnostics: Arc::clone(&binding.agent().diagnostics),
            current_theme: Mutex::new(window::current_window_theme()),
            monitor_streams: RuntimeStreamRegistry::default(),
            window_streams: RuntimeStreamRegistry::default(),
            monitor_topology_snapshot: RuntimeSnapshotCache::default(),
            cursor_hidden: Mutex::new(false),
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

    /// Borrow the agent resource table captured by this runtime.
    pub(crate) fn resource_table(&self) -> &ResourceTable {
        // safety: the agent owns the resource table for the lifetime of the runtime state
        unsafe { &*self.resources.0 }
    }

    /// Borrow the runtime world captured by this runtime.
    pub(crate) fn world(&self) -> &World {
        // safety: the world outlives the runtime state for the lifetime of the agent
        unsafe { &*self.world.0 }
    }

    /// Register one host-owned ingress observer for this runtime.
    fn register_runtime_ingress(self: &Arc<Self>, context: &BindingCallContext) {
        let runtime_id = context.agent().runtime_id;

        let observer = self
            .runtime_ingress_observer
            .get_or_init(|| {
                Arc::new(AppKitIngressObserver {
                    runtime_state: Arc::downgrade(self),
                })
            })
            .clone();
        let observer: Arc<dyn RuntimeIngressObserver> = observer;

        register_runtime_ingress_observer(runtime_id.0, &observer);
    }
}

/// Return the shared runtime registry for process-global monitor callbacks.
fn monitor_callback_runtimes() -> &'static Mutex<HashMap<u64, Weak<AppKitRuntimeState>>> {
    APPKIT_MONITOR_CALLBACK_RUNTIMES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register the process-global CoreGraphics display callback once.
fn ensure_monitor_callback_registered() {
    APPKIT_MONITOR_CALLBACK_REGISTRATION.get_or_init(|| {
        let callback: CGDisplayReconfigurationCallBack = Some(handle_display_reconfiguration);
        let status =
            unsafe { CGDisplayRegisterReconfigurationCallback(callback, std::ptr::null_mut()) };

        // surface callback registration failures through tracing so the backend does not fail silently
        if status != CGError(0) {
            tracing::warn!(
                target: "destack.runtime.display.appkit",
                "CGDisplayRegisterReconfigurationCallback failed with status {:?}",
                status
            );
        }
    });
}

/// Register one runtime for process-global monitor reconfiguration callbacks.
fn register_monitor_runtime(runtime_id: u64, runtime_state: &Arc<AppKitRuntimeState>) {
    let mut registry = monitor_callback_runtimes()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // prune dead runtimes before updating the keyed registration
    registry.retain(|_, weak| weak.strong_count() > 0);
    registry.insert(runtime_id, Arc::downgrade(runtime_state));

    ensure_monitor_callback_registered();
}

/// Handle one process-global display reconfiguration callback from CoreGraphics.
unsafe extern "C-unwind" fn handle_display_reconfiguration(
    _display: CGDirectDisplayID,
    flags: CGDisplayChangeSummaryFlags,
    _user_info: *mut std::ffi::c_void,
) {
    // ignore the begin-configuration marker because no stable topology exists yet
    if flags.contains(CGDisplayChangeSummaryFlags::BeginConfigurationFlag) {
        return;
    }

    let mut registry = monitor_callback_runtimes()
        .lock()
        .unwrap_or_else(|error| error.into_inner());

    // retain only live runtimes while broadcasting the topology refresh
    registry.retain(|_, weak| {
        let Some(runtime_state) = weak.upgrade() else {
            return false;
        };

        if let Err(error) = appkit_event::publish_monitor_topology_deltas(&runtime_state) {
            super::core::warn_callback_error(
                runtime_state.as_ref(),
                "destack.display.monitor.reconfigurationCallback",
                error.as_ref(),
            );
        }

        true
    });
}

/// Return runtime-owned AppKit state for this binding call.
pub(crate) fn runtime_state(binding: &BindingCallContext) -> Arc<AppKitRuntimeState> {
    let runtime_state = binding
        .agent()
        .platform_state
        .display
        .appkit_runtime_state(|| AppKitRuntimeState::from_context(binding));

    {
        let runtime_id = binding.agent().runtime_id;
        register_monitor_runtime(runtime_id.0, &runtime_state);
    }
    runtime_state.register_runtime_ingress(binding);

    runtime_state
}

/// Run one closure against main-thread AppKit backend state.
pub(crate) fn with_main_thread_state<F, R>(
    runtime_state: &Arc<AppKitRuntimeState>,
    callback: F,
) -> R
where
    F: Send + FnOnce(&RefCell<AppKitMainThreadState>) -> R,
    R: Send,
{
    runtime_state.main_thread_state.get_on_main(callback)
}

/// Resolve one opened native host window payload.
pub(crate) fn with_window_host<F, R>(
    runtime_state: &Arc<AppKitRuntimeState>,
    window: resource::WindowHandle,
    operation: &'static str,
    callback: F,
) -> RuntimeResult<R>
where
    F: Send + FnOnce(&AppKitWindowHost) -> RuntimeResult<R>,
    R: Send,
{
    with_main_thread_state(runtime_state, |state| {
        let state = state.borrow();
        let host = state.windows.get(&window).ok_or_else(|| {
            core_platform::io_not_found(
                operation,
                format!("window handle {} was not found", window.0.0),
            )
        })?;

        callback(host)
    })
}
