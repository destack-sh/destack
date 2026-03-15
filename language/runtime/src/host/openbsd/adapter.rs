use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;
use crate::runtime::world::RuntimeId;

/// OpenBSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct OpenBsdHost;

impl OpenBsdHost {
    /// Create one OpenBSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for OpenBsdHost {
    fn platform(&self) -> Platform {
        Platform::OpenBsd
    }

    fn session_capabilities(&self, _runtime_id: RuntimeId) -> PlatformCapabilitySet {
        unix_request_capabilities()
    }

    fn submit_request(
        &self,
        _context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        submit_unix_request(request)
    }
}
