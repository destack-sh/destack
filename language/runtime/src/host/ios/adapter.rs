use crate::diagnostic::RuntimeResult;
use crate::host::apple::core::message as apple_message;
use crate::host::core::error::not_supported;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostSessionId};
use crate::host::ios::capability;
use crate::host::ios::request::submit_request;
use crate::host::{HostAdapter, Platform};
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
        apple_message::is_process_main_context()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        // service one ready slice of the Apple run loop
        apple_message::process_ingress_ready(true);

        Ok(())
    }

    fn session_capabilities(&self, host_session_id: HostSessionId) -> PlatformCapabilitySet {
        capability::session_capabilities(host_session_id)
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context, &request)? {
            return Ok(outcome);
        }

        Err(not_supported(request.operation_name()))
    }
}
