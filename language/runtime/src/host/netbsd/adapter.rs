use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostRuntimeId};
use crate::host::unix::{submit_unix_request, unix_request_capabilities};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// NetBSD host implementation.
#[derive(Debug, Default)]
pub(crate) struct NetBsdHost;

impl NetBsdHost {
    /// Create one NetBSD host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for NetBsdHost {
    fn platform(&self) -> Platform {
        Platform::NetBsd
    }

    fn session_capabilities(&self, _host_runtime_id: HostRuntimeId) -> PlatformCapabilitySet {
        unix_request_capabilities()
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        submit_unix_request(context, request)
    }
}
