use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// Linux host implementation.
#[derive(Debug, Default)]
pub(crate) struct LinuxHost;

impl LinuxHost {
    /// Create one Linux host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for LinuxHost {
    fn platform(&self) -> Platform {
        Platform::Linux
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
