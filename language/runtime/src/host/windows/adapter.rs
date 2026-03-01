use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{
    HostState, HostStateRegistration, default_host_capabilities, register_host_state,
};
use crate::host::{HostAdapter, HostLifecycleState, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// Windows host implementation.
#[derive(Debug)]
pub(crate) struct WindowsHost {
    /// Shared host state used for event ingestion and state updates.
    state: Arc<HostState>,
    /// Shared registration guard for callback routing.
    registration: HostStateRegistration,
}

impl WindowsHost {
    /// Create one Windows host.
    pub(crate) fn new() -> Self {
        let state = Arc::new(HostState::new());
        state.push_lifecycle(HostLifecycleState::Initializing);
        let registration = register_host_state(HostPlatform::Windows, &state);

        Self {
            state,
            registration,
        }
    }
}

impl HostAdapter for WindowsHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Windows
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

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }

    fn state(&self) -> &Arc<HostState> {
        &self.state
    }
}
