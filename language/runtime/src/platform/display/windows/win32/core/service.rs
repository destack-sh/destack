use std::sync::Arc;

use crate::platform::service::affinity::{ServiceAffinity, ServiceHostLoop};
use crate::platform::service::executor::host::HostLoopExecutor;
use crate::platform::service::global_service;
use crate::runtime::BindingCallContext;

use super::runtime::Win32RuntimeState;

/// Process-global Win32 display service.
pub(crate) struct Win32DisplayService {
    /// Host-loop executor for this service.
    executor: HostLoopExecutor,
}

impl Win32DisplayService {
    /// The host-affinity domain for the Win32 display service.
    pub(crate) const AFFINITY: ServiceAffinity =
        ServiceAffinity::HostLoop(ServiceHostLoop::WindowsMessageLoop);

    /// Create one process-global Win32 display service.
    fn new() -> Self {
        Self {
            executor: HostLoopExecutor::new(
                "platform.display.win32",
                ServiceHostLoop::WindowsMessageLoop,
            ),
        }
    }

    /// Register one live runtime with the Win32 host ingress loop.
    pub(crate) fn register_runtime(
        &self,
        context: &BindingCallContext,
        runtime_state: &Arc<Win32RuntimeState>,
    ) {
        let runtime_id = context.agent().runtime_id;
        let runtime_state = runtime_state.clone();

        self.executor
            .call_loop("destack.display.service.win32.register", move || {
                runtime_state.register_runtime_ingress(runtime_id);
                Ok(())
            })
            .expect("Win32 display service registration should succeed");
    }
}

/// Return one shared Win32 display service.
pub(crate) fn win32_display_service() -> Arc<Win32DisplayService> {
    debug_assert!(matches!(
        Win32DisplayService::AFFINITY,
        ServiceAffinity::HostLoop(ServiceHostLoop::WindowsMessageLoop)
    ));

    global_service(|| Ok(Win32DisplayService::new()))
        .expect("Win32 display service initialization should succeed")
}
