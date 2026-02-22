use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::host::{HostAdapter, HostEvent, HostPlatform, HostServices};
use crate::runtime::poller::HostPollerWakeHandle;

/// NetBSD host adapter scaffold.
#[derive(Debug, Default, Clone)]
pub(crate) struct NetBsdHostAdapter {
    /// Service surfaces exposed by this adapter.
    services: HostServices,
}

impl NetBsdHostAdapter {
    /// Create one NetBSD host adapter scaffold.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl HostAdapter for NetBsdHostAdapter {
    fn platform(&self) -> HostPlatform {
        HostPlatform::NetBsd
    }

    fn poll_events(&self, _timeout_nanos: Option<u64>) -> RuntimeResult<Vec<HostEvent>> {
        Ok(Vec::new())
    }

    fn wake_handle(&self) -> Option<Arc<dyn HostPollerWakeHandle>> {
        None
    }

    fn services(&self) -> &HostServices {
        &self.services
    }
}
