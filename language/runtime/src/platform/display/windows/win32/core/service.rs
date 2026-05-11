use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::service::Service;
use crate::runtime::service::executor::host::HostExecutor;
use crate::runtime::{BindingCallContext, ExecutionAffinity, ExecutionMode, ExecutionPolicy};

use super::runtime::Win32RuntimeState;

/// Process-global Win32 display service.
pub(crate) struct Win32DisplayService {
    /// Host-loop executor for this service.
    executor: HostExecutor,
}

impl Win32DisplayService {
    /// Create one process-global Win32 display service.
    fn new() -> RuntimeResult<Self> {
        Ok(Self {
            executor: HostExecutor::new(
                "platform.display.win32",
                ExecutionAffinity::WindowsMessageLoop,
            )?,
        })
    }

    /// Register one live runtime with the Win32 host ingress loop.
    pub(crate) fn register_runtime(
        &self,
        context: &BindingCallContext,
        runtime_state: &Arc<Win32RuntimeState>,
    ) {
        let host_session_id = context.host().host_session_id();
        let runtime_state = runtime_state.clone();

        self.executor
            .call_loop("destack.display.service.win32.register", move || {
                runtime_state.register_runtime_ingress(host_session_id)?;
                Ok(())
            })
            .expect("Win32 display service registration should succeed");
    }
}

impl Service for Win32DisplayService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::process(ExecutionMode::Host)
        .with_affinity(ExecutionAffinity::WindowsMessageLoop);
}

/// Return one shared Win32 display service.
pub(crate) fn win32_display_service() -> Arc<Win32DisplayService> {
    Win32DisplayService::global(Win32DisplayService::new)
        .expect("Win32 display service initialization should succeed")
}
