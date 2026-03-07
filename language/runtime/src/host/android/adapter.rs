use std::sync::Arc;

use destack_workspace::PlatformHostOptions;

use super::{message as android_message, unregister_android_bindings};
use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostAdapterState, default_host_capabilities};
use crate::host::{HostAdapter, HostPlatform, HostPollOutcome};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::poller::HostPollerWakeHandle;
use crate::runtime::world::RuntimeId;

/// Android host implementation.
#[derive(Debug)]
pub(crate) struct AndroidHost {
    /// Shared adapter state used for event ingestion and callback routing.
    state: HostAdapterState,
}

impl AndroidHost {
    /// Create one Android host.
    pub(crate) fn new(runtime_id: RuntimeId) -> Self {
        Self {
            state: HostAdapterState::new(
                HostPlatform::Android,
                runtime_id,
                Some(unregister_android_bindings),
            ),
        }
    }
}

impl HostAdapter for AndroidHost {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Android
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
        false
    }

    fn process_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Android looper
        Ok(android_message::process_ingress_ready(true))
    }

    fn host_capabilities(&self) -> PlatformCapabilitySet {
        default_host_capabilities(self.platform())
    }
}

#[cfg(test)]
#[path = "tests/adapter.rs"]
mod tests;
