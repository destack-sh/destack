use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// Haiku host implementation.
#[derive(Debug, Default)]
pub(crate) struct HaikuHost;

impl HaikuHost {
    /// Create one Haiku host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for HaikuHost {
    fn platform(&self) -> Platform {
        Platform::Haiku
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
