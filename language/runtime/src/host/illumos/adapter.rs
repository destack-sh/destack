use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// Illumos host implementation.
#[derive(Debug, Default)]
pub(crate) struct IllumosHost;

impl IllumosHost {
    /// Create one Illumos host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for IllumosHost {
    fn platform(&self) -> Platform {
        Platform::Illumos
    }

    fn session_capabilities(&self) -> PlatformCapabilitySet {
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
