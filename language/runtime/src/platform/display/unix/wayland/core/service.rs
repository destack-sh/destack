use std::sync::Arc;

use crate::runtime::service::Service;
use crate::runtime::service::executor::inline::InlineExecutor;
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy};

use super::runtime::WaylandRuntimeState;

/// Process-global wayland display service.
pub(crate) struct WaylandDisplayService {
    /// Caller-thread executor for this service.
    executor: InlineExecutor,
}

impl WaylandDisplayService {
    /// Create one process-global wayland display service.
    fn new() -> Self {
        Self {
            executor: InlineExecutor::new("platform.display.wayland"),
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
                runtime_state.register_runtime_ingress(context)?;
                Ok(())
            })
            .expect("wayland display service registration should succeed");
    }
}

impl Service for WaylandDisplayService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return one shared wayland display service.
pub(crate) fn wayland_display_service() -> Arc<WaylandDisplayService> {
    WaylandDisplayService::global(|| Ok(WaylandDisplayService::new()))
        .expect("wayland display service initialization should succeed")
}
