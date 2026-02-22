use std::sync::Arc;

use crate::diagnostic::RuntimeResult;
use crate::runtime::host::{HostAdapter, HostEvent, HostPlatform, HostServices};
use crate::runtime::poller::HostPollerWakeHandle;

/// illumos host adapter scaffold.
#[derive(Debug, Default, Clone)]
pub(crate) struct IllumosHostAdapter {
    /// Service surfaces exposed by this adapter.
    services: HostServices,
}

impl IllumosHostAdapter {
    /// Create one illumos host adapter scaffold.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl HostAdapter for IllumosHostAdapter {
    fn platform(&self) -> HostPlatform {
        HostPlatform::Illumos
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
