use std::sync::Arc;

use crate::runtime::BindingCallContext;
use crate::runtime::process::service::executor::host::HostExecutor;
use crate::runtime::process::{ExecutionAffinity, ExecutionMode, ExecutionPolicy, GlobalService};

use super::runtime::Win32RuntimeState;

/// Process-global Win32 display service.
pub(crate) struct Win32DisplayService {
    /// Host-loop executor for this service.
    executor: HostExecutor,
}

impl Win32DisplayService {
    /// Create one process-global Win32 display service.
    fn new() -> Self {
        Self {
            executor: HostExecutor::new(
                "platform.display.win32",
                ExecutionAffinity::WindowsMessageLoop,
            ),
        }
    }

    /// Register one live runtime with the Win32 host ingress loop.
    pub(crate) fn register_runtime(
        &self,
        context: &BindingCallContext,
        runtime_state: &Arc<Win32RuntimeState>,
    ) {
        let host_runtime_id = context.host().host_runtime_id();
        let runtime_state = runtime_state.clone();

        self.executor
            .call_loop("destack.display.service.win32.register", move || {
                runtime_state.register_runtime_ingress(host_runtime_id)?;
                Ok(())
            })
            .expect("Win32 display service registration should succeed");
    }
}

impl GlobalService for Win32DisplayService {
    const POLICY: ExecutionPolicy = ExecutionPolicy::global(ExecutionMode::Host)
        .with_affinity(ExecutionAffinity::WindowsMessageLoop);
}

/// Return one shared Win32 display service.
pub(crate) fn win32_display_service() -> Arc<Win32DisplayService> {
    Win32DisplayService::global(|| Ok(Win32DisplayService::new()))
        .expect("Win32 display service initialization should succeed")
}
