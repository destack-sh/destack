use crate::diagnostic::RuntimeResult;
use crate::host::os::unix::request;
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
};
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

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    fn session_capabilities(&self, _host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        request::request_capabilities()
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
