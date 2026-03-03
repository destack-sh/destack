use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostAdapterState, HostState, default_host_capabilities, not_supported};
use crate::host::{HostAdapter, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// FreeBSD host implementation.
#[derive(Debug)]
pub(crate) struct FreeBsdHost {
    /// Shared adapter state used for event ingestion and callback routing.
    state: HostAdapterState,
}

impl FreeBsdHost {
    /// Create one FreeBSD host.
    pub(crate) fn new() -> Self {
        Self {
            state: HostAdapterState::new(HostPlatform::FreeBsd, None),
        }
    }
}

impl HostAdapter for FreeBsdHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::FreeBsd
    }

    fn poll_events(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollOutcome> {
        self.state.state().poll_events(timeout_nanos)
    }

    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        Some(self.state.state().wake_handle())
    }

    fn configure_host_options(&self, host_options: &PlatformHostOptions) {
        self.state.state().configure_host_options(host_options);
    }

    fn callback_runtime_id(&self) -> Option<u64> {
        Some(self.state.callback_runtime_id())
    }

    fn pump_pending_thread_messages(&self, ignore_quit_message: bool) -> RuntimeResult<bool> {
        let _ = ignore_quit_message;
        Err(not_supported(
            "runtime.host.platform.pumpPendingThreadMessages",
        ))
    }

    fn run_blocking_thread_message_loop(&self) -> RuntimeResult<()> {
        Err(not_supported(
            "runtime.host.platform.runBlockingThreadMessageLoop",
        ))
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }

    fn state(&self) -> &Arc<HostState> {
        self.state.state()
    }
}
