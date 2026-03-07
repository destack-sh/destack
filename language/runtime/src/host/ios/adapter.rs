use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use crate::diagnostic::RuntimeResult;
use crate::host::apple::message as apple_message;
use crate::host::core::{HostAdapterState, default_host_capabilities};
use crate::host::{HostAdapter, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;
use crate::runtime::world::RuntimeId;

/// iOS host implementation.
#[derive(Debug)]
pub(crate) struct IosHost {
    /// Shared adapter state used for event ingestion and callback routing.
    state: HostAdapterState,
}

impl IosHost {
    /// Create one iOS host.
    pub(crate) fn new(runtime_id: RuntimeId) -> Self {
        Self {
            state: HostAdapterState::new(HostPlatform::IOS, runtime_id, None),
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

    fn is_process_main_context(&self) -> bool {
        apple_message::is_process_main_context()
    }

    fn process_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Apple run loop
        Ok(apple_message::process_ingress_ready(true))
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
