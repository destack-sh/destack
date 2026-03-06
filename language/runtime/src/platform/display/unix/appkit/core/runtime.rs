use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Condvar, Mutex, OnceLock, Weak};

use dispatch2::{MainThreadBound, run_on_main};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSImage, NSWindow};
use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayChangeSummaryFlags, CGDisplayReconfigurationCallBack,
    CGDisplayRegisterReconfigurationCallback, CGError,
};

use crate::diagnostic::{AgentDiagnosticStore, RuntimeResult};
use crate::host::apple::message::{self as apple_message, AppleThreadMessageObserver};
use crate::platform::display::WindowPosition;
use crate::platform::{ResourceTable, core as core_platform, resource};
use crate::runtime::BindingCallContext;

use super::super::event::{
    DisplayEventRecord, MonitorEventBinding, WindowEventBinding, WindowEventRecord,
};
use super::super::model::{AppKitWindowBinding, MonitorSnapshot};
use super::super::{event, window};
use super::delegate::AppKitWindowDelegate;

/// Runtime-owned monitor-event log state.
#[derive(Debug)]
pub(crate) struct AppKitMonitorEventRuntimeState {
    /// Sequence for the first retained live record.
    pub(crate) first_sequence: u64,
    /// Sequence to assign to the next live record.
    pub(crate) next_sequence: u64,
    /// Retained live monitor-event records.
    pub(crate) records: VecDeque<DisplayEventRecord>,
}

impl Default for AppKitMonitorEventRuntimeState {
    /// Build one empty monitor-event runtime state.
    fn default() -> Self {
        Self {
            first_sequence: 1,
            next_sequence: 1,
            records: VecDeque::new(),
        }
    }
}

/// Runtime-owned window-event log state.
#[derive(Debug)]
pub(crate) struct AppKitWindowEventRuntimeState {
    /// Sequence for the first retained live record.
    pub(crate) first_sequence: u64,
    /// Sequence to assign to the next live record.
    pub(crate) next_sequence: u64,
    /// Retained live window-event records.
    pub(crate) records: VecDeque<WindowEventRecord>,
}

impl Default for AppKitWindowEventRuntimeState {
    /// Build one empty window-event runtime state.
    fn default() -> Self {
        Self {
            first_sequence: 1,
            next_sequence: 1,
            records: VecDeque::new(),
        }
    }
}

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
    /// The runtime binding payload for this window.
    pub(crate) binding: Arc<Mutex<AppKitWindowBinding>>,
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

/// Host-owned thread-message observer for one AppKit runtime.
#[derive(Debug)]
struct AppKitThreadMessageObserver {
    /// Weak runtime state used for post-pump reconciliation.
    runtime_state: Weak<AppKitRuntimeState>,
}

impl AppleThreadMessageObserver for AppKitThreadMessageObserver {
    /// Reconcile runtime-owned window state after one host message pump.
    fn did_pump_thread_messages(&self) {
        let Some(runtime_state) = self.runtime_state.upgrade() else {
            return;
        };

        // keep callback-driven window state coherent after the host services AppKit work
        if let Err(error) = window::reconcile_all_host_window_bindings(&runtime_state) {
            super::core::warn_callback_error(
                runtime_state.as_ref(),
                "destack.display.window.hostPump",
                error.as_ref(),
            );
        }
    }
}

/// Runtime-owned AppKit display backend state.
pub(crate) struct AppKitRuntimeState {
    /// Main-thread state for native AppKit objects.
    pub(crate) main_thread_state: MainThreadBound<RefCell<AppKitMainThreadState>>,
    /// Agent resource table used for callback-owned display-handle updates.
    pub(crate) resources: AppKitResourceTableRef,
    /// Cached display handles keyed by stable AppKit display identifier.
    pub(crate) display_handle_cache: Mutex<HashMap<String, resource::DisplayHandle>>,
    /// Runtime-owned monitor-event log.
    pub(crate) monitor_events: Mutex<AppKitMonitorEventRuntimeState>,
    /// Wake signal for monitor-event readers.
    pub(crate) monitor_event_signal: Condvar,
    /// Runtime-owned window-event log.
    pub(crate) window_events: Mutex<AppKitWindowEventRuntimeState>,
    /// Wake signal for window-event readers.
    pub(crate) window_event_signal: Condvar,
    /// Runtime diagnostics store for callback and best-effort lanes.
    pub(crate) diagnostics: Arc<AgentDiagnosticStore>,
    /// Monitor-event subscribers for this runtime.
    pub(crate) monitor_event_registry: Mutex<Vec<Weak<MonitorEventBinding>>>,
    /// Window-event subscribers for this runtime.
    pub(crate) window_event_registry: Mutex<Vec<Weak<WindowEventBinding>>>,
    /// Cached monitor topology snapshot for monitor-event delta publication.
    pub(crate) monitor_topology_snapshot: Mutex<Option<Vec<MonitorSnapshot>>>,
    /// Whether the process-global AppKit cursor is currently hidden.
    pub(crate) cursor_hidden: Mutex<bool>,
    /// Registered host-owned observer for post-pump AppKit reconciliation.
    thread_message_observer: OnceLock<Arc<AppKitThreadMessageObserver>>,
}

/// Shared runtime registry for process-global CoreGraphics display callbacks.
static APPKIT_MONITOR_CALLBACK_RUNTIMES: OnceLock<Mutex<Vec<Weak<AppKitRuntimeState>>>> =
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
            display_handle_cache: Mutex::new(HashMap::new()),
            monitor_events: Mutex::new(AppKitMonitorEventRuntimeState::default()),
            monitor_event_signal: Condvar::new(),
            window_events: Mutex::new(AppKitWindowEventRuntimeState::default()),
            window_event_signal: Condvar::new(),
            diagnostics: Arc::clone(&binding.agent().diagnostic),
            monitor_event_registry: Mutex::new(Vec::new()),
            window_event_registry: Mutex::new(Vec::new()),
            monitor_topology_snapshot: Mutex::new(None),
            cursor_hidden: Mutex::new(false),
            thread_message_observer: OnceLock::new(),
        }
    }
}

/// Borrow the agent resource table captured by this runtime.
pub(crate) fn resource_table(runtime_state: &AppKitRuntimeState) -> &ResourceTable {
    // safety: the agent owns the resource table for the lifetime of the runtime state
    unsafe { &*runtime_state.resources.0 }
}

/// Return the shared runtime registry for process-global monitor callbacks.
fn monitor_callback_runtimes() -> &'static Mutex<Vec<Weak<AppKitRuntimeState>>> {
    APPKIT_MONITOR_CALLBACK_RUNTIMES.get_or_init(|| Mutex::new(Vec::new()))
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
fn register_monitor_runtime(runtime_state: &Arc<AppKitRuntimeState>) {
    let identity = Arc::as_ptr(runtime_state) as usize;
    let mut registry = monitor_callback_runtimes()
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let mut found_existing = false;

    // retain only live runtimes and avoid duplicate registrations
    registry.retain(|weak| {
        let Some(strong) = weak.upgrade() else {
            return false;
        };

        let strong_identity = Arc::as_ptr(&strong) as usize;
        if strong_identity == identity {
            found_existing = true;
        }

        true
    });

    if !found_existing {
        registry.push(Arc::downgrade(runtime_state));
    }

    ensure_monitor_callback_registered();
}

/// Register one host-owned thread-message observer for this runtime.
fn register_thread_message_runtime(
    binding: &BindingCallContext,
    runtime_state: &Arc<AppKitRuntimeState>,
) {
    let Some(runtime_id) = binding.host().callback_runtime_id() else {
        return;
    };

    let observer = runtime_state
        .thread_message_observer
        .get_or_init(|| {
            Arc::new(AppKitThreadMessageObserver {
                runtime_state: Arc::downgrade(runtime_state),
            })
        })
        .clone();
    let observer: Arc<dyn AppleThreadMessageObserver> = observer;

    apple_message::register_thread_message_observer(runtime_id, &observer);
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
    registry.retain(|weak| {
        let Some(runtime_state) = weak.upgrade() else {
            return false;
        };

        if let Err(error) = event::publish_monitor_topology_deltas(&runtime_state) {
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

    register_monitor_runtime(&runtime_state);
    register_thread_message_runtime(binding, &runtime_state);

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
