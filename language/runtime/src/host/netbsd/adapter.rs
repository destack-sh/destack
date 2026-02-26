use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{
    HostBridge, HostBridgeRegistration, HostState, default_host_capabilities, register_host_bridge,
};
use crate::host::{Host, HostLifecycleState, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// NetBSD host implementation.
#[derive(Debug)]
pub(crate) struct NetBsdHost {
    /// Shared callback bridge used for event ingestion and state updates.
    bridge: Arc<HostBridge>,
    /// Shared registration guard for callback routing.
    registration: HostBridgeRegistration,
}

impl NetBsdHost {
    /// Create one NetBSD host.
    pub(crate) fn new() -> Self {
        let state = Arc::new(HostState::new());
        let bridge = Arc::new(HostBridge::new(state));
        bridge.push_lifecycle(HostLifecycleState::Initializing);
        let registration = register_host_bridge(HostPlatform::NetBsd, &bridge);

        Self {
            bridge,
            registration,
        }
    }
}

impl Host for NetBsdHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::NetBsd
    }

    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        self.bridge.poll_events(timeout_nanos)
    }

    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        Some(self.bridge.wake_handle())
    }

    fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        self.bridge.configure_host_options(host_options);
    }

    fn callback_runtime_id(&self) -> Option<u64> {
        Some(self.registration.runtime_id())
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }

    fn state(&self) -> &Arc<HostState> {
        self.bridge.state()
    }
}
