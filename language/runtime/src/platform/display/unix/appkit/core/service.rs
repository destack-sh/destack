use std::sync::{Arc, Mutex, OnceLock};

use objc2_core_graphics::{
    CGDirectDisplayID, CGDisplayChangeSummaryFlags, CGDisplayReconfigurationCallBack,
    CGDisplayRegisterReconfigurationCallback, CGError,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::display::unix::appkit::event as appkit_event;
use crate::runtime::service::executor::host::HostExecutor;
use crate::runtime::service::{ProcessSubscriberRegistry, Service};
use crate::runtime::{
    BindingCallContext, ExecutionAffinity, ExecutionMode, ExecutionPolicy, WorkerId,
};

use super::core::warn_callback_error;
use super::runtime::AppKitRuntimeState;

/// Process-global AppKit display service.
pub(crate) struct AppKitDisplayService {
    /// Host-loop executor for this service.
    executor: HostExecutor,
    /// Mutable service state.
    state: Arc<AppKitDisplayServiceState>,
}

/// Mutable AppKit display service state.
struct AppKitDisplayServiceState {
    /// Registered runtime subscribers for monitor topology callbacks.
    monitor_callback_runtimes: Mutex<ProcessSubscriberRegistry<WorkerId, AppKitRuntimeState>>,
    /// Guard that installs the CoreGraphics callback once.
    monitor_callback_registration: OnceLock<()>,
}

impl AppKitDisplayService {
    /// Create one process-global AppKit display service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            executor: HostExecutor::new("platform.display.appkit", ExecutionAffinity::MainThread)?,
            state: Arc::new(AppKitDisplayServiceState {
                monitor_callback_runtimes: Mutex::new(ProcessSubscriberRegistry::default()),
                monitor_callback_registration: OnceLock::new(),
            }),
        })
    }

    /// Register one live runtime with the AppKit host loop.
    pub(crate) fn register_runtime(
        &self,
        binding: &BindingCallContext,
        runtime_state: &Arc<AppKitRuntimeState>,
    ) {
        let worker_id = binding.worker().id;
        let host_session_id = binding.host().host_session_id();
        let state = self.state.clone();
        let runtime_state = runtime_state.clone();

        // keep runtime registration on the AppKit main thread
        self.executor
            .call_loop("destack.display.service.appkit.register", move || {
                let mut registry = state
                    .monitor_callback_runtimes
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                registry.register(worker_id, &runtime_state);

                drop(registry);

                ensure_monitor_callback_registered(&state);
                runtime_state.register_runtime_ingress(host_session_id)?;
                Ok(())
            })
            .expect("AppKit display service registration should succeed");
    }

    /// Return one snapshot of the live monitor callback runtimes.
    fn monitor_callback_runtimes_snapshot(&self) -> Vec<Arc<AppKitRuntimeState>> {
        let state = self.state.clone();

        self.executor
            .call_loop(
                "destack.display.monitor.reconfigurationCallback",
                move || {
                    let mut registry = state
                        .monitor_callback_runtimes
                        .lock()
                        .unwrap_or_else(|error| error.into_inner());

                    Ok(registry.snapshot())
                },
            )
            .expect("AppKit display monitor snapshot should succeed")
    }
}

impl Service for AppKitDisplayService {
    const POLICY: ExecutionPolicy =
        ExecutionPolicy::process(ExecutionMode::Host).with_affinity(ExecutionAffinity::MainThread);
}

/// Return one shared AppKit display service.
pub(crate) fn appkit_display_service() -> Arc<AppKitDisplayService> {
    AppKitDisplayService::global(AppKitDisplayService::new)
        .expect("AppKit display service initialization should succeed")
}

/// Return one live AppKit display service when it has already been initialized.
///
/// This is for late CoreGraphics callbacks that may race service teardown.
fn active_appkit_display_service() -> Option<Arc<AppKitDisplayService>> {
    AppKitDisplayService::active()
}

/// Register the process-global CoreGraphics callback once.
fn ensure_monitor_callback_registered(state: &AppKitDisplayServiceState) {
    state.monitor_callback_registration.get_or_init(|| {
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

    // ignore callbacks that race display service teardown
    let Some(service) = active_appkit_display_service() else {
        return;
    };

    // broadcast one topology refresh to every live runtime subscriber
    for runtime_state in service.monitor_callback_runtimes_snapshot() {
        if let Err(error) = appkit_event::publish_monitor_topology_deltas(&runtime_state) {
            warn_callback_error(
                runtime_state.as_ref(),
                "destack.display.monitor.reconfigurationCallback",
                error.as_ref(),
            );
        }
    }
}
