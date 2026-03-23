use crate::diagnostic::RuntimeResult;
use crate::host::core::{
    HostRequest, HostRequestContext, HostRequestOutcome, HostSessionContext, HostSessionId,
};
use crate::host::linux::{ingress, request};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::{PlatformCapability, PlatformCapabilitySet};

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

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        PlatformCapabilitySet::new()
    }

    fn session_capabilities(&self, _host_runtime_id: HostSessionId) -> PlatformCapabilitySet {
        let mut capabilities = request::request_capabilities();
        capabilities.extend_capabilities([
            PlatformCapability::OsBackgroundControl,
            PlatformCapability::OsBackgroundRead,
        ]);

        capabilities
    }

    fn process_runtime_ingress(&self, context: &HostSessionContext) -> RuntimeResult<()> {
        ingress::service_linux_ingress(context)
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
