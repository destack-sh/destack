use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::{HostAdapterState, HostState, default_host_capabilities};
use crate::host::{HostAdapter, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;

/// iOS host implementation.
#[derive(Debug)]
pub(crate) struct IosHost {
    /// Shared adapter state used for event ingestion and callback routing.
    state: HostAdapterState,
}

impl IosHost {
    /// Create one iOS host.
    pub(crate) fn new() -> Self {
        Self {
            state: HostAdapterState::new(HostPlatform::IOS, Some(apple_message::cleanup_runtime)),
        }
    }
}

impl HostAdapter for IosHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::IOS
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
        let runtime_id = Some(self.state.callback_runtime_id());
        let dispatched = apple_message::pump_pending_thread_messages_for_runtime(
            runtime_id,
            ignore_quit_message,
        );
        Ok(dispatched)
    }

    fn run_blocking_thread_message_loop(&self) -> RuntimeResult<()> {
        apple_message::run_blocking_thread_message_loop();
        Ok(())
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }

    fn state(&self) -> &Arc<HostState> {
        self.state.state()
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
