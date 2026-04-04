use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::os::apple::ingress::r#loop as apple_ingress_loop;
use crate::host::os::ios::capability;
use crate::host::os::ios::request::submit_request;
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
};
use crate::runtime::capability::PlatformCapabilitySet;

/// iOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct IosHost;

impl IosHost {
    /// Create one iOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for IosHost {
    fn platform(&self) -> Platform {
        Platform::IOS
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        capability::static_capabilities()
    }

    fn is_process_main_context(&self) -> bool {
        apple_run_loop::is_process_main_context()
    }

    fn advance_native_ingress(&self) -> RuntimeResult<()> {
        // drain one ready slice of the Apple run loop
        apple_ingress_loop::drain_ready_ingress();

        Ok(())
    }

    fn session_capabilities(&self, host_session_id: HostSessionId) -> PlatformCapabilitySet {
        capability::session_capabilities(host_session_id)
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context, &request)? {
            return Ok(outcome);
        }

        Err(not_supported(request.operation_name()))
    }
}
