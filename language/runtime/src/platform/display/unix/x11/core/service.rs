use std::sync::Arc;

use crate::runtime::service::Service;
use crate::runtime::service::executor::inline::InlineExecutor;
use crate::runtime::{BindingCallContext, ExecutionMode, ExecutionPolicy};

use super::runtime::X11RuntimeState;

/// Process-global x11 display service.
pub(crate) struct X11DisplayService {
    /// Caller-thread executor for this service.
    executor: InlineExecutor,
}

impl X11DisplayService {
    /// Create one process-global x11 display service.
    fn new() -> Self {
        Self {
            executor: InlineExecutor::new("platform.display.x11"),
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
                runtime_state.register_runtime_ingress(context)?;
                Ok(())
            })
            .expect("x11 display service registration should succeed");
    }
}

impl Service for X11DisplayService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Inline);
}

/// Return one shared x11 display service.
pub(crate) fn x11_display_service() -> Arc<X11DisplayService> {
    X11DisplayService::global(|| Ok(X11DisplayService::new()))
        .expect("x11 display service initialization should succeed")
}
