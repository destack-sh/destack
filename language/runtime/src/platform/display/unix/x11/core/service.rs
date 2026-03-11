use std::sync::Arc;

use crate::platform::service::affinity::ServiceAffinity;
use crate::platform::service::executor::CallerThreadExecutor;
use crate::platform::service::global_service;
use crate::runtime::BindingCallContext;

use super::runtime::X11RuntimeState;

/// Process-global x11 display service.
pub(crate) struct X11DisplayService {
    /// Caller-thread executor for this service.
    executor: CallerThreadExecutor,
}

impl X11DisplayService {
    /// The host-affinity domain for the x11 display service.
    pub(crate) const AFFINITY: ServiceAffinity = ServiceAffinity::CallerThread;

    /// Create one process-global x11 display service.
    fn new() -> Self {
        Self {
            executor: CallerThreadExecutor::new("platform.display.x11"),
        }
    }

    /// Register one live runtime with the x11 ingress loop.
    pub(crate) fn register_runtime(
        &self,
        context: &BindingCallContext,
        runtime_state: &Arc<X11RuntimeState>,
    ) {
        let runtime_state = runtime_state.clone();

        // keep runtime registration explicit and one-time through the service boundary
        self.executor
            .call("destack.display.service.x11.register", move || {
                runtime_state.register_runtime_ingress(context);
                Ok(())
            })
            .expect("x11 display service registration should succeed");
    }
}

/// Return one shared x11 display service.
pub(crate) fn x11_display_service() -> Arc<X11DisplayService> {
    debug_assert!(matches!(
        X11DisplayService::AFFINITY,
        ServiceAffinity::CallerThread
    ));

    global_service(|| Ok(X11DisplayService::new()))
        .expect("x11 display service initialization should succeed")
}
