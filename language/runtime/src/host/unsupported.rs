use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostState, HostStateRegistration, not_supported, register_host_state};
use crate::host::{HostAdapter, HostLifecycleState, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// Unsupported host implementation.
#[derive(Debug)]
pub(crate) struct UnsupportedHost {
    /// Shared host state used for event ingestion and state updates.
    state: Arc<HostState>,
    /// Shared registration guard for callback routing.
    registration: HostStateRegistration,
}

impl UnsupportedHost {
    /// Create one unsupported host.
    pub(crate) fn new() -> Self {
        let state = Arc::new(HostState::new());
        state.push_lifecycle(HostLifecycleState::Initializing);
        let registration = register_host_state(HostPlatform::Universal, &state, None);

        Self {
            state,
            registration,
        }
    }
}

impl HostAdapter for UnsupportedHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Universal
    }

    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        self.state.poll_events(timeout_nanos)
    }

    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        Some(self.state.wake_handle())
    }

    fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        self.state.configure_host_options(host_options);
    }

    fn callback_runtime_id(&self) -> Option<u64> {
        Some(self.registration.runtime_id())
    }

    fn pump_pending_thread_messages(&self, ignore_quit_message: bool) -> RuntimeResult<bool> {
        let _ = ignore_quit_message;
        Err(not_supported(
            "runtime.host.platform.pumpPendingThreadMessages",
        ))
    }

    fn run_blocking_thread_message_loop(&self) -> RuntimeResult<()> {
        Err(not_supported(
            "runtime.host.platform.runBlockingThreadMessageLoop",
        ))
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    fn state(&self) -> &Arc<HostState> {
        &self.state
    }
}
