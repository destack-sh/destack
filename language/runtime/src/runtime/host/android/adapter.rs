use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::host::{HostAdapter, HostEvent, HostPlatform, HostServices};
use crate::runtime::poller::HostPollerWakeHandle;

/// Android host adapter scaffold.
#[derive(Debug, Default, Clone)]
pub(crate) struct AndroidHostAdapter {
    /// Service surfaces exposed by this adapter.
    services: HostServices,
}

impl AndroidHostAdapter {
    /// Create one Android host adapter scaffold.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl HostAdapter for AndroidHostAdapter {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Android
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
