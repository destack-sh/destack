use std::sync::Arc;

use crate::runtime::BindingCallContext;
use crate::runtime::process::service::affinity::ServiceAffinity;
use crate::runtime::process::service::executor::caller::CallerThreadExecutor;
use crate::runtime::process::service::global_service;

use super::runtime::WaylandRuntimeState;

/// Process-global wayland display service.
pub(crate) struct WaylandDisplayService {
    /// Caller-thread executor for this service.
    executor: CallerThreadExecutor,
}

impl WaylandDisplayService {
    /// The host-affinity domain for the wayland display service.
    pub(crate) const AFFINITY: ServiceAffinity = ServiceAffinity::CallerThread;

    /// Create one process-global wayland display service.
    fn new() -> Self {
        Self {
            executor: CallerThreadExecutor::new("platform.display.wayland"),
        }
    }

    /// Register one live runtime with the wayland ingress loop.
    pub(crate) fn register_runtime(
        &self,
        context: &BindingCallContext,
        runtime_state: &Arc<WaylandRuntimeState>,
    ) {
        let runtime_state = runtime_state.clone();

        // keep runtime registration explicit and one-time through the service boundary
        self.executor
            .call("destack.display.service.wayland.register", move || {
                runtime_state.register_runtime_ingress(context);
                Ok(())
            })
            .expect("wayland display service registration should succeed");
    }
}

/// Return one shared wayland display service.
pub(crate) fn wayland_display_service() -> Arc<WaylandDisplayService> {
    debug_assert!(matches!(
        WaylandDisplayService::AFFINITY,
        ServiceAffinity::CallerThread
    ));

    global_service(|| Ok(WaylandDisplayService::new()))
        .expect("wayland display service initialization should succeed")
}
