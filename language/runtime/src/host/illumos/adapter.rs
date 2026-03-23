use crate::diagnostic::RuntimeResult;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostSessionId};
use crate::host::unix::request;
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

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    fn session_capabilities(&self, _host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        request::request_capabilities()
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
