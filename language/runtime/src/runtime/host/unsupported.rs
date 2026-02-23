use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::runtime::host::core::{
    HostBridge, HostBridgeRegistration, HostServiceState, register_host_bridge,
};
use crate::runtime::host::{
    HostAdapter, HostEvent, HostInterruptionService, HostLifecycleService, HostLifecycleState,
    HostPermissionService, HostPlatform, HostServices, HostWindowService,
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
        let service_state = Arc::new(HostServiceState::new());
        let bridge = Arc::new(HostBridge::new(service_state));
        bridge.push_lifecycle(HostLifecycleState::Initializing);
        let registration = register_host_bridge(HostPlatform::Universal, &bridge);
        let services = build_unsupported_services(bridge.service_state());

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
fn build_unsupported_services(service_state: &Arc<HostServiceState>) -> HostServices {
    let lifecycle_service: Arc<dyn HostLifecycleService> = service_state.clone();
    let window_service: Arc<dyn HostWindowService> = service_state.clone();
    let permission_service: Arc<dyn HostPermissionService> = service_state.clone();
    let interruption_service: Arc<dyn HostInterruptionService> = service_state.clone();

    HostServices::default()
        .with_lifecycle(lifecycle_service)
        .with_window(window_service)
        .with_permission(permission_service)
        .with_interruption(interruption_service)
}
