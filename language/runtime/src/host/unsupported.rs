use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostBridge, HostBridgeRegistration, HostStateStore, register_host_bridge};
use crate::host::{
    HostAdapter, HostEvent, HostLifecycleState, HostPermissionService, HostPlatform, HostServices,
    HostStateReader,
};
use crate::runtime::poller::HostPollerWakeHandle;

/// Unsupported host adapter implementation.
#[derive(Debug)]
pub(crate) struct UnsupportedHostAdapter {
    /// Shared callback bridge used for event ingestion and state updates.
    bridge: Arc<HostBridge>,
    /// Shared registration guard for callback routing.
    registration: HostBridgeRegistration,
    /// Service surfaces exposed by this adapter.
    services: HostServices,
}

impl UnsupportedHostAdapter {
    /// Create one unsupported host adapter.
    pub(crate) fn new() -> Self {
        let state_store = Arc::new(HostStateStore::new());
        let bridge = Arc::new(HostBridge::new(state_store));
        bridge.push_lifecycle(HostLifecycleState::Initializing);
        let registration = register_host_bridge(HostPlatform::Universal, &bridge);
        let services = build_unsupported_services(bridge.state_store());

        Self {
            bridge,
            registration,
            services,
        }
    }
}

impl HostAdapter for UnsupportedHostAdapter {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Universal
    }

    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
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

    fn take_dropped_event_count(&self) -> u64 {
        self.bridge.take_dropped_event_count()
    }

    fn services(&self) -> &HostServices {
        &self.services
    }
}

/// Build unsupported host service trait surfaces from one shared state object.
fn build_unsupported_services(state_store: &Arc<HostStateStore>) -> HostServices {
    let state_service: Arc<dyn HostStateReader> = state_store.clone();
    let permission_service: Arc<dyn HostPermissionService> = state_store.clone();

    HostServices::default()
        .with_state(state_service)
        .with_permission(permission_service)
}
