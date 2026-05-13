use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::diagnostic::RuntimeResult;
use crate::host::poller::HostPoller;
use crate::host::{Host, HostSession};
use crate::runtime::runtime::poller_for_backend;
use crate::world::RuntimeId;
use destack_workspace::{PollerBackend, RuntimeOptions};

/// Immutable runtime host settings captured for restore and fork.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeHostOptions {
    /// Captured poller backend for runtime restore.
    pub poller_backend: PollerBackend,
}

impl RuntimeHostOptions {
    /// Build one runtime host settings snapshot from runtime options.
    pub(crate) fn from_runtime_options(options: &RuntimeOptions) -> Self {
        Self {
            poller_backend: options.scheduler_options().poller_backend,
        }
    }

    /// Build one host session for the given runtime id.
    pub(crate) fn host_session(&self, host: Arc<dyn Host>, runtime_id: RuntimeId) -> HostSession {
        HostSession::new(host, runtime_id)
    }

    /// Build one poller for this runtime host settings snapshot.
    pub(crate) fn poller(&self) -> RuntimeResult<Box<dyn HostPoller>> {
        poller_for_backend(self.poller_backend)
    }
}
